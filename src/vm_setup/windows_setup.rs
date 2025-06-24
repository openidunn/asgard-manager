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

/// Asynchronously runs a virtual machine configured by `setup`.
///
/// This function creates a new partition (VM), configures it according to
/// the `VmSetup` parameters, allocates guest memory, creates virtual CPUs,
/// and runs each vCPU in parallel tasks.
///
/// # Arguments
///
/// * `setup` - A `VmSetup` instance containing VM configuration such as CPU cores and memory size.
///
/// # Returns
///
/// * `Ok(())` if the VM ran successfully (all vCPUs halted properly).
/// * `Err(String)` if any step fails during partition creation, setup, memory allocation,
///    vCPU creation, or execution.
///
/// # Notes
///
/// - Uses Windows Hypervisor Platform APIs to create and manage partitions and vCPUs.
/// - Runs each virtual CPU on a separate blocking task using `tokio::task::spawn_blocking`.
///
pub async fn run_vm(setup: VmSetup) -> Result<(), String> {
    // 1. Create a new partition (virtual machine container)
    let partition: Partition = match create_partition() {
        Ok(p) => p,
        Err(e) => return Err(format!("Partition creation failed: {:?}", e)),
    };

    // 2. Set the number of virtual processors for the partition
    let processor_count = setup.get_cpu_cores_count() as u32;
    if let Err(e) = set_processor_count_property(&partition, setup.get_cpu_cores_count()) {
        return Err(format!("Failed to set processor count: {:?}", e));
    }

    // 3. Setup the partition (apply all configured properties)
    if let Err(e) = setup_partition(&partition) {
        return Err(format!("Failed to setup partition: {:?}", e));
    }

    // 4. Allocate and map guest physical memory for the partition
    if let Err(e) = allocate_partition_memory(partition.get_whv_partition_handle(), setup.get_memory_size() as u64) {
        return Err(format!("Failed to allocate and map guest memory: {:?}", e));
    }

    // 5. Create and run virtual CPUs (vCPUs) concurrently, one per CPU core
    let mut handlers: Vec<tokio::task::JoinHandle<Result<String, String>>> = Vec::new();
    for cpu_id in 0..setup.get_cpu_cores_count() {
        // Clone the partition handle for each task (handle is Copy)
        let ph = partition.get_whv_partition_handle().clone();

        // Spawn a blocking task for each vCPU to avoid blocking async runtime
        handlers.push(task::spawn_blocking(move || -> Result<String, String> {
            // Create the vCPU within the partition with the given CPU id
            if let Err(e) = create_vcpu(ph, cpu_id as u32) {
                return Err(format!("Failed to create VCPU {}: {:?}", cpu_id, e));
            };

            // Enter an execution loop for this vCPU
            loop {
                // Run the vCPU until it exits for some reason
                let exit_ctx = match run_vcpu(ph, cpu_id) {
                    Ok(exit_ctx) => exit_ctx,
                    Err(e) => return Err(format!("VCPU {} failed to run: {:?}", cpu_id, e))
                };

                // Check the reason the vCPU stopped execution
                match exit_ctx.ExitReason {
                    WHvRunVpExitReasonX64Halt => {
                        // VCPU executed HLT instruction; clean halt
                        return Ok(format!("VCPU {} halted (HLT)", cpu_id))
                    }
                    WHvRunVpExitReasonNone => {
                        // Invalid or unexpected exit state
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
                        // Catch any other unknown exit reasons
                        return Err(format!("VCPU {} unknown exit reason {:?}", cpu_id, other))
                    }
                }
            }
        }));
    }

    // Await all vCPU tasks and collect their results
    for h in handlers {
        match h.await {
            Ok(Ok(msg)) => println!("Success: {}", msg), // Task succeeded, vCPU halted properly
            Ok(Err(err)) => return Err(err),             // Task returned an error from vCPU execution
            Err(e) => return Err(format!("Task join error: {}", e)), // Tokio task join error
        }
    }

    Ok(())
}
