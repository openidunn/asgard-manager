//! Virtual Machine setup and execution utilities for macOS using applevisor and Tokio.
//!
//! This module provides the `VmSetup` struct for configuring VM memory and CPU cores,
//! and the `run_vm` async function to launch and manage a VM instance.

use applevisor::*;
use std::{result::Result};
use tokio;
use tokio::sync::Mutex;
use std::sync::Arc;

/// Configuration for a Virtual Machine instance.
pub struct VmSetup {
    /// Size of VM memory in bytes.
    memory: usize,
    /// Number of CPU cores to allocate to the VM.
    cpu_cores_count: u32
}

impl VmSetup {
    /// Create a new `VmSetup`.
    ///
    /// # Arguments
    /// * `mega_bytes` - Memory size in megabytes.
    /// * `cpu_cores_count` - Number of CPU cores (defaults to 2 if 0).
    pub fn new(mega_bytes: u32, cpu_cores_count: u32) -> VmSetup {
        let cpu_cores_to_set = if cpu_cores_count == 0 || cpu_cores_count == 1 {
            2
        } else {
            cpu_cores_count
        };
        VmSetup {memory: 1024 * 1024 * mega_bytes as usize, cpu_cores_count: cpu_cores_to_set}
    }
    /// Get the configured memory size in bytes.
    pub fn get_memory_size(&self) -> usize {
        self.memory
    }
    /// Get the configured number of CPU cores.
    pub fn get_cpu_cores_count(&self) -> u32 {
        self.cpu_cores_count
    }
}

/// Asynchronously run a Virtual Machine with the given setup on macOS.
///
/// # Arguments
/// * `setup` - The VM configuration to use.
///
/// # Returns
/// * `Ok(())` if the VM runs successfully.
/// * `Err(String)` if any error occurs during setup or execution.
//Running VM on macos
#[cfg(target_os = "macos")]
pub async fn run_vm(setup: VmSetup) -> Result<(), String> {

    // Create a new VirtualMachine instance, wrapped in Arc<Mutex<...>> for thread safety.
    let mut _vm = match VirtualMachine::new() {
        Ok(vm) => Arc::new(Mutex::new(vm)),
        Err(e) => return Err(format!("Failed to create VM: {}", e))
    };
    // Allocate guest memory for the VM.
    let mut mem = match Mapping::new(setup.get_memory_size()) {
        Ok(mem) => mem,
        Err(_) => return Err("Failed to create memory".to_string())
    };
    // Map the memory region at address 0x4000 with RWX permissions.
    if let Err(_) = mem.map(0x4000, MemPerms::RWX) {
        return Err("Failed to map memory".to_string());
    };

    // Spawn a blocking task for each virtual CPU core.
    let mut handlers: Vec<tokio::task::JoinHandle<Result<String, String>>> = Vec::new();
    for i in 0..setup.cpu_cores_count {
        
        let handle = tokio::task::spawn_blocking(move || {
            // Create a new VCPU instance.
            let vcpu = match Vcpu::new() {
                Ok(vcpu) => vcpu,
                Err(_) => {
                    return Err("Failed to create VCPU".to_string());
                }
            };
            // Set up debug exception and register traps for the VCPU.
            if let Err(_) = vcpu.set_trap_debug_exceptions(true) {
                return Err("Failed to set trap debug exceptions for CPU".to_string());
            }
            if let Err(_) = vcpu.set_trap_debug_reg_accesses(true) {
                return Err("Failed to set trap debug register accesses for CPU".to_string());
            }
            // Set the program counter (PC) register to the start address.
            if let Err(_) = vcpu.set_reg(Reg::PC, 0x4000)  {
                return Err("Failed to set trap debug instruction executions for CPU".to_string());
            }
            // Start running the VCPU.
            if let Err(_) = vcpu.run() {
                return Err(format!("Failed to run VCPU {}", i));
            }

            // Main VCPU event loop: handle VM exits and exceptions.
            loop {
                let exit = vcpu.get_exit_info();
                match exit.reason {
                    ExitReason::CANCELED => {
                        return Ok(format!("VCPU {} stopped", i))
                    },
                    ExitReason::EXCEPTION => {
                        let exception = exit.exception;
                        let syndrome = exception.syndrome;
                        let ec = (syndrome >> 26) & 0x3F;
                        let iss = syndrome & 0xFFFFFF;

                        match ec {
                            0x0D => {
                                // General Protection Fault
                                return Err(format!("VCPU {} encountered General Protection Fault", i));
                            }
                            0x15 => { // Data Abort
                                let va = exception.virtual_address;
                                let pa = exception.physical_address;
                                return Err(format!(
                                    "VCPU {} Data Abort at VA: 0x{:x}, PA: 0x{:x}, ISS: 0x{:x}",
                                    i, va, pa, iss
                                ));
                            }
                            _ => {
                                // Other exception
                                return Err(format!(
                                    "VCPU {} exited with exception EC=0x{:x}, ISS=0x{:x}",
                                    i, ec, iss
                                ));
                            }
                        }
                    }
                    ExitReason::VTIMER_ACTIVATED => {
                        return Err(format!("VCPU {} exited due to virtual timer activation", i));
                    }
                    ExitReason::UNKNOWN => {
                        return Err(format!("VCPU {} exited due to unknown reason", i));
                    }
                };
            }
        });

        handlers.push(handle);
    }

    // Await all VCPU tasks and check for errors.
    for handle in handlers {
        if let Err(_) = handle.await {
            return Err("Failed to join VCPU task".to_string());
        };
    }
    
    Ok(())
}