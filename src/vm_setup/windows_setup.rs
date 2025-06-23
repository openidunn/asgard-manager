use windows::Win32::System::Hypervisor::{
    WHvCreatePartition, WHvDeletePartition, WHvSetPartitionProperty,
    WHvPartitionPropertyCodeProcessorCount, WHvMapGpaRange, WHV_MAP_GPA_RANGE_FLAGS,
    WHvMapGpaRangeFlagRead, WHvMapGpaRangeFlagWrite, WHvMapGpaRangeFlagExecute,
    WHvSetupPartition, WHvCreateVirtualProcessor, WHvRunVirtualProcessor,
    WHV_RUN_VP_EXIT_CONTEXT, WHV_PARTITION_HANDLE,
};
use crate::vm_setup::setup_utils::VmSetup;
use super::super::windows_bindings::*;
use tokio::task;

pub async fn run_vm(setup: VmSetup) -> Result<(), String> {
    // 1. Create partition
    let partition: WHV_PARTITION_HANDLE = match create_partition() {
        Ok(p) => p,
        Err(e) => return Err(format!("Partition creation failed: {:?}", e)),
    };

    // 2. Set processor count
    let processor_count = setup.get_cpu_cores_count() as u32;
    if let Err(e) = set_processor_count_property(partition, setup.get_cpu_cores_count()) {
        return Err(format!("Failed to set processor count: {:?}", e));
    }

    if let Err(e) = setup_partition(partition) {
        return Err(format!("Failed to setup partition: {:?}", e));
    }

    // 3. Allocate & map guest memory
    if let Err(e) = allocate_partition_memory(partition, setup.get_memory_size() as u64) {
        return Err(format!("Failed to allocate and map guest memory: {:?}", e));
    }

    // 5. Create & run VCPUs
    let mut handlers: Vec<tokio::task::JoinHandle<Result<String, String>>> = Vec::new();
    for cpu_id in 0..setup.get_cpu_cores_count() {
        let ph = partition.clone();
        handlers.push(task::spawn_blocking(move || -> Result<String, String> {
            if let Err(e) = create_vcpu(ph, cpu_id as u32) {
                return Err(format!("Failed to create VCPU {}: {:?}", cpu_id, e));
            };

            loop {
                let exit_ctx = match run_vcpu(ph, cpu_id) {
                    Ok(exit_ctx) => exit_ctx,
                    Err(e) => return Err(format!("VCPU {} failed to run: {:?}", cpu_id, e))
                };

                match exit_ctx.ExitReason {
                    WHvRunVpExitReasonX64Halt => {
                        return Ok(format!("VCPU {} halted (HLT)", cpu_id))
                    }
                    WHvRunVpExitReasonNone => {
                        return Err(format!("VCPU {} exited with NONE (invalid state)", cpu_id))
                    }
                    WHvRunVpExitReasonMemoryAccess => {
                        return Err(format!("VCPU {} memory access exit", cpu_id))
                    }
                    WHvRunVpExitReasonX64IoPortAccess => {
                        return Err(format!("VCPU {} IO port access exit", cpu_id))
                    }
                    WHvRunVpExitReasonX64MsrAccess => {
                        return Err(format!("VCPU {} MSR access exit", cpu_id))
                    }
                    WHvRunVpExitReasonX64Cpuid => {
                        return Err(format!("VCPU {} CPUID exit (unhandled CPUID)", cpu_id))
                    }
                    WHvRunVpExitReasonException => {
                        return Err(format!("VCPU {} caused exception", cpu_id))
                    }
                    WHvRunVpExitReasonUnsupportedFeature => {
                        return Err(format!("VCPU {} unsupported feature exit", cpu_id))
                    }
                    other => {
                        return Err(format!("VCPU {} unknown exit reason {:?}", cpu_id, other))
                    }
                }
            }
        }));
    }

    for h in handlers {
        match h.await {
            Ok(Ok(msg)) => println!("Success: {}", msg),
            Ok(Err(err)) => return Err(err),
            Err(e) => return Err(format!("Task join error: {}", e)),
        }
    }

    let _ = delete_partition(partition);

    Ok(())
}
