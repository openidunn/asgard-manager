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