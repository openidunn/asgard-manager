//! Linux VM setup and execution utilities using KVM and Tokio.
//!
//! This module provides the `run_vm` async function to launch and manage a KVM-based VM instance
//! with the configuration provided by `VmSetup`.

use kvm_ioctls::{Kvm, VcpuExit, Vcpu};
use crate::vm_setup::setup_utils::VmSetup;
use vm_memory::{GuestAddress, GuestMemoryMmap};
use kvm_bindings;

/// Asynchronously runs a virtual machine using KVM with the provided setup.
///
/// # Arguments
/// * `setup` - The VM configuration to use (memory size, CPU count, etc).
///
/// # Returns
/// * `Ok(())` if the VM runs successfully.
/// * `Err(String)` if any error occurs during setup or execution.
pub async fn run_vm(setup: VmSetup) -> Result<(), String> {
    // Create a new KVM instance
    let kvm = match Kvm::new() {
        Ok(kvm) => kvm,
        Err(e) => return Err(format!("Failed to create KVM instance: {}", e)),
    };
    // Create a new VM from the KVM instance
    let vm = match kvm.create_vm() {
        Ok(vm) => vm,
        Err(e) => return Err(format!("Failed to create VM: {}", e))
    };

    // Set up guest memory at a specific address
    let load_addr = GuestAddress(0x4000);
    let guest_memory = match GuestMemoryMmap::from_ranges(&[(load_addr, setup.get_memory_size())]) {
        Ok(mem) => mem,
        Err(e) => return Err(format!("Failed to create guest memory: {}", e)),
    };

    let host_addr = match guest_memory.get_host_address(load_addr) {
        Ok(addr) => addr,
        Err(e) => return Err(format!("Failed to get host address for guest memory: {}", e)),
    };

    // Register the memory region with the VM
    if let Err(e) = vm.set_memory_region(kvm_bindings::kvm_userspace_memory_region {
        slot: 0,
        guest_phys_addr: 0x4000,
        memory_size: setup.get_memory_size() as u64,
        userspace_addr: host_addr,
        flags: 0,
    }) {
        return Err(format!("Failed to set memory region: {}", e));
    };

    // Spawn a blocking task for each virtual CPU core
    let handlers: Vec<tokio::task::JoinHandle<Result<String, String>>> =
        Vec::with_capacity(setup.get_cpu_cores_count() as usize);
    for cpu_id in 0..setup.get_cpu_cores_count() {
        // Create a VCPU for this core
        let vcpu = match vm.create_vcpu(cpu_id) {
            Ok(vcpu) => vcpu,
            Err(e) => return Err(format!("Failed to create VCPU {}: {}", cpu_id, e)),
        };

        // Set initial register state for the VCPU
        let mut regs = match vcpu.get_regs() {
            Ok(regs) => regs,
            Err(e) => return Err(format!("Failed to get VCPU {} registers: {}", cpu_id, e)),
        };

        regs.rip = 0x1000;
        regs.rflags = 0x2;

        if let Err(e) = vcpu.set_regs(&regs) {
            return Err(format!("Failed to set VCPU {} registers: {}", cpu_id, e));
        };

        // Spawn a blocking task to run the VCPU event loop
        let handler = tokio::task::spawn_blocking(move || {
            loop {
                match vcpu.run() {
                    Ok(exit_reason) => {
                        // Handle different VCPU exit reasons
                        match exit_reason {
                            VcpuExit::Hlt => { 
                                return Ok(format!("VCPU {} exited with HLT instruction", cpu_id));
                             },
                            VcpuExit::IoIn( port, data ) => { 
                                return Err(format!("VCPU {} encountered IO in at port {:x} with data {:?}", cpu_id, port, data));
                             },
                            VcpuExit::IoOut( port, data) => { 
                                return Err(format!("VCPU {} encountered IO out at port {:x} with data {:?}", cpu_id, port, data));
                             },
                            VcpuExit::MmioRead ( address, data ) => { 
                                return Err(format!("VCPU {} encountered MMIO read at address {:x}", cpu_id, address));
                             },
                            VcpuExit::MmioWrite ( address, data ) => { 
                                return Err(format!("VCPU {} encountered MMIO write at address {:x}", cpu_id, address));
                             },
                            VcpuExit::Shutdown => { 
                                return Ok(format!("VCPU {} exited gracefully", cpu_id));
                             },
                            VcpuExit::InternalError => { 
                                return Err(format!("VCPU {} encountered an internal error", cpu_id));
                             },
                            VcpuExit::SystemEvent (..) => { 
                                return Err(format!("VCPU {} encountered a system event", cpu_id));
                             },
                            _ => { 
                                return Err(format!("Unhandled VCPU exit reason: {:?}", exit_reason));
                            }
                        }
                    },
                    Err(e) => {
                        return Err(format!("VCPU {} encountered an error: {}", cpu_id, e));
                    }
                }
            }
        });
        handlers.push(handler);
    }

    // Await all VCPU tasks and handle their results
    for handler in handlers {
        match handler.await {
            Ok(Ok(msg)) => println!("VCPU completed: {}", msg),
            Ok(Err(err)) => return Err(err),
            Err(e) => return Err(format!("Task join error: {}", e)),
        }
    }

    Ok(())
}