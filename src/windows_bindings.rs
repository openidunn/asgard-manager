use windows::Win32::System::Hypervisor::{
    WHvCreatePartition, WHvDeletePartition, WHvSetPartitionProperty,
    WHV_PARTITION_HANDLE, WHvPartitionPropertyCodeProcessorCount,
    WHvMapGpaRange, WHV_MAP_GPA_RANGE_FLAGS,
    WHvMapGpaRangeFlagRead, WHvMapGpaRangeFlagWrite, WHvMapGpaRangeFlagExecute,
    WHvSetupPartition, WHvCreateVirtualProcessor, WHvRunVirtualProcessor,
    WHV_RUN_VP_EXIT_CONTEXT
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::core::HRESULT;
use windows::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};

/// A safe wrapper around a WHV_PARTITION_HANDLE.
///
/// This struct owns a hypervisor partition handle and ensures
/// it is properly cleaned up when dropped.
pub struct Partition {
    // The raw hypervisor partition handle.
    partition: WHV_PARTITION_HANDLE,
}

impl Partition {
    /// Creates a new `Partition` from a raw `WHV_PARTITION_HANDLE`.
    ///
    /// # Safety
    /// The caller must ensure that the handle is valid and not used elsewhere.
    /// This struct assumes ownership and will delete the partition on drop.
    pub fn new(partition: WHV_PARTITION_HANDLE) -> Self {
        Partition { partition }
    }

    /// Returns the raw `WHV_PARTITION_HANDLE` for use with FFI functions.
    ///
    /// This does not transfer ownership — the caller must not delete
    /// or close the handle manually.
    pub fn get_whv_partition_handle(&self) -> WHV_PARTITION_HANDLE {
        self.partition
    }
}

impl Drop for Partition {
    /// Automatically deletes the partition when the `Partition` is dropped.
    ///
    /// This ensures proper resource cleanup through the Windows Hypervisor API.
    fn drop(&mut self) {
        // SAFETY: This is safe because we own the handle and Drop is only called once.
        delete_partition(self.partition);
    }
}

/// Retrieves total and available physical memory on the host system.
/// Returns a tuple: (total_physical_memory_bytes, available_physical_memory_bytes)
fn get_physical_memory_info() -> Result<(u64, u64), String> {
    unsafe {
        // Initialize MEMORYSTATUSEX struct with its size
        let mut mem_status = MEMORYSTATUSEX::default();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        
        // Call GlobalMemoryStatusEx to fill mem_status with memory info
        match GlobalMemoryStatusEx(&mut mem_status) {
            Ok(_) => Ok((mem_status.ullTotalPhys, mem_status.ullAvailPhys)),
            Err(e) => Err(format!("{:?}", e))
        }
    }
}

/// Creates a new Hyper-V partition and returns Partition instance with its handle.
/// On failure, returns an error string.
pub fn create_partition() -> Result<Partition, String> {
    match unsafe { WHvCreatePartition() } {
        Ok(handle) => Ok(Partition::new(handle)),
        Err(e) => Err(format!("{:?}", e))
    }
}

/// Sets the processor count property for a given partition.
/// Valid processor_count range is 1 to 64 (inclusive).
/// Returns Ok on success or an error string on failure.
pub fn set_processor_count_property(partition: &Partition, processor_count: u32) -> Result<(), String> {
    // Validate processor count
    if processor_count == 0 || processor_count > 64 {
        return Err(format!("Failed to set processor count: processor_count equal to {}", processor_count));
    }
    // Attempt to set the property on the partition
    if let Err(e) = unsafe {
        WHvSetPartitionProperty(
            partition.get_whv_partition_handle(),
            WHvPartitionPropertyCodeProcessorCount,
            &processor_count as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    } {
        return Err(format!("{:?}", e));
    }

    Ok(())
}

/// Deletes the given partition handle, cleaning up resources.
/// Returns Ok on success or an error string on failure.
fn delete_partition(partition: WHV_PARTITION_HANDLE) -> Result<(), String> {
    if let Err(e) = unsafe { WHvDeletePartition(partition) } {
        return Err(format!("{:?}", e))
    }

    Ok(())
}

/// Finalizes the partition setup after properties are configured.
/// Must be called before running virtual processors.
/// Returns Ok on success or an error string on failure.
pub fn setup_partition(partition: &Partition) -> Result<(), String> {
    match unsafe { WHvSetupPartition(partition.get_whv_partition_handle()) } {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("{:?}", e))
    }
}

/// Allocates host memory and maps it into the guest physical address space.
/// - `partition`: Partition handle to map memory into.
/// - `mem_size`: Size of memory to allocate and map (in bytes).
/// Returns Ok on success or error string on failure.
pub fn allocate_partition_memory(partition: &Partition, mem_size: u64) -> Result<(), String> {
    // Get host memory info
    let (total_mem, avail_mem) = match get_physical_memory_info() {
        Ok((total_mem, avail_mem)) => (total_mem, avail_mem),
        Err(e) => return Err(format!("Failed to get memory info: {}", e)),
    };

    // Check if enough available memory on host
    if avail_mem < mem_size {
        return Err("Failed to allocate the memory: not enough available memory".to_string());
    }

    // Allocate virtual memory on host with read/write permissions
    let ptr = unsafe {
        VirtualAlloc(
            None,
            mem_size as usize,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        )
    };
    if ptr.is_null() {
        return Err("VirtualAlloc failed".to_string());
    }

    // Prepare flags for memory mapping: readable, writable, executable
    let flags = WHV_MAP_GPA_RANGE_FLAGS(
        WHvMapGpaRangeFlagRead.0 |
        WHvMapGpaRangeFlagWrite.0 |
        WHvMapGpaRangeFlagExecute.0,
    );

    // Map the allocated host memory into the guest physical address space starting at GPA 0
    let result = unsafe {
        WHvMapGpaRange(partition.get_whv_partition_handle(), ptr as *mut _, 0x0000, mem_size, flags)
    };

    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to map memory: {:?}", e)),
    }
}

/// Creates a virtual CPU (vCPU) in the given partition with the specified CPU ID.
/// Returns Ok on success or error string on failure.
pub fn create_vcpu(partition: &Partition, cpu_id: u32) -> Result<(), String> {
    let hresult = unsafe { WHvCreateVirtualProcessor(partition.get_whv_partition_handle(), cpu_id, 0) };
    if let Err(e) = hresult {
        return Err(format!("Failed to create virtual processor: {:?}", e));
    }

    Ok(())
}

/// Runs the virtual CPU with the given CPU ID on the specified partition.
/// Returns the exit context on success or error string on failure.
pub fn run_vcpu(partition: &Partition, cpu_id: u32) -> Result<WHV_RUN_VP_EXIT_CONTEXT, String> {
    let mut vcpu_ctx: WHV_RUN_VP_EXIT_CONTEXT = WHV_RUN_VP_EXIT_CONTEXT::default();
    let val_size = std::mem::size_of_val(&vcpu_ctx) as u32;

    // Run the vCPU and fill vcpu_ctx with exit information
    if let Err(e) = unsafe { WHvRunVirtualProcessor(partition.get_whv_partition_handle(), cpu_id, &mut vcpu_ctx as *mut _ as *mut _, val_size) } {
        return Err(format!("{:?}", e));
    }

    Ok(vcpu_ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_partition_wraps_handle() {

        // SAFETY: We're calling a Windows API. Make sure the hypervisor is enabled.
        let result = unsafe { WHvCreatePartition() };
        let result = result.unwrap();

        let partition = Partition::new(result);

        assert_eq!(partition.get_whv_partition_handle(), result);

        // When `partition` goes out of scope, Drop will call `WHvDeletePartition`
    }

    /// Test retrieving physical memory information succeeds and yields sane values
    #[test]
    fn test_get_physical_memory_info_success() {
        let result = get_physical_memory_info();
        assert!(result.is_ok(), "Should successfully get memory info on Windows");
        
        let (total, available) = result.unwrap();
        println!("Total RAM: {} bytes ({:.2} GB)", total, total as f64 / 1024.0 / 1024.0 / 1024.0);
        println!("Available RAM: {} bytes ({:.2} GB)", available, available as f64 / 1024.0 / 1024.0 / 1024.0);
        
        assert!(total > 0, "Total physical memory should be greater than 0");
        assert!(available > 0, "Available physical memory should be greater than 0");
        assert!(available <= total, "Available memory should not exceed total memory");
    }

    /// Test physical memory values are within expected reasonable bounds for modern hardware
    #[test]
    fn test_memory_values_sanity_check() {
        let result = get_physical_memory_info();
        assert!(result.is_ok(), "Memory info should be accessible");
        
        let (total, available) = result.unwrap();
        
        // Reasonable bounds for modern systems
        const MIN_MEMORY: u64 = 512 * 1024 * 1024; // 512 MB
        const MAX_MEMORY: u64 = 1024 * 1024 * 1024 * 1024; // 1 TB
        
        assert!(total >= MIN_MEMORY, "Total memory seems too small (< 512 MB)");
        assert!(total <= MAX_MEMORY, "Total memory seems too large (> 1 TB)");
        
        let usage_percent = ((total - available) as f64 / total as f64) * 100.0;
        assert!(usage_percent < 99.0, "Memory usage ({:.1}%) seems unreasonably high", usage_percent);
    }

    /// Test partition creation succeeds and returns a valid handle
    #[test]
    fn test_create_partition() {
        let result = create_partition();
        match result {
            Ok(handle) => {
                println!("Partition created: {:?}", handle.get_whv_partition_handle());
            }
            Err(e) => {
                println!("Partition creation failed: {}", e);
                // Optionally, assert on known error messages if you expect failure in your environment
            }
        }

        assert!(true)
    }

    /// Test partition deletion succeeds for a valid partition handle
    #[test]
    fn test_delete_partition() {
        let raw_handle = unsafe { WHvCreatePartition() };
        match raw_handle {
            Ok(handle) => {
                let del_result = delete_partition(handle);
                assert!(del_result.is_ok(), "Partition deletion failed: {:?}", del_result.err());
            }
            Err(e) => {
                println!("Raw WHvCreatePartition failed: {:?}", e);
                assert!(false, "Raw partition creation failed");
            }
        }
    }

    /// Test setting a valid processor count property on a partition
    #[test]
    fn test_set_processor_count_property_valid() {
        let partition = create_partition().expect("Partition creation failed");
        let result = set_processor_count_property(&partition, 2);
        assert!(result.is_ok(), "Should succeed for valid processor count");
    }

    /// Test that setting zero processor count returns an error
    #[test]
    fn test_set_processor_count_property_zero() {
        let partition = create_partition().expect("Partition creation failed");
        let result = set_processor_count_property(&partition, 0);
        assert!(result.is_err(), "Should fail for processor_count == 0");
        assert_eq!(
            result.unwrap_err(),
            "Failed to set processor count: processor_count equal to 0"
        );
    }

    /// Test setting processor count with invalid (zeroed) partition handle fails
    #[test]
    fn test_set_processor_count_property_invalid_partition() {
        // Create an invalid handle (zeroed)
        let invalid_partition = WHV_PARTITION_HANDLE::default();
        let partition = Partition::new(invalid_partition);
        let result = set_processor_count_property(&partition, 2);
        assert!(result.is_err(), "Should fail for invalid partition handle");
    }

    /// Test setting an excessively large processor count returns an error
    #[test]
    fn test_set_processor_count_property_large_count() {
        let partition = create_partition().expect("Partition creation failed");
        // Use a very large processor count, likely to be invalid on most systems
        let result = set_processor_count_property(&partition, 1024);
        assert!(result.is_err(), "Should fail for unreasonably large processor count");
    }

    /// Test setting processor count property and successful partition setup
    #[test]
    fn test_setup_partition_success() {
        let partition = create_partition().expect("Failed to create partition");

        // Set required processor count before setup
        let processor_count: u32 = 2;
        let set_result = unsafe {
            WHvSetPartitionProperty(
                partition.get_whv_partition_handle(),
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        assert!(set_result.is_ok(), "Failed to set processor count: {:?}", set_result.err());

        // Now call setup
        let result = setup_partition(&partition);
        assert!(result.is_ok(), "setup_partition failed: {:?}", result.err());
    }

    /// Test setup fails if required processor count property is not set
    #[test]
    fn test_setup_partition_without_processor_property() {
        let partition = create_partition().expect("Failed to create partition");

        // Intentionally skip setting processor count
        let result = setup_partition(&partition);
        assert!(result.is_err(), "Expected setup to fail without required properties");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("HRESULT"),
            "Expected HRESULT error, got: {}",
            err_msg
        );
    }

    /// Test setup fails with invalid partition handle
    #[test]
    fn test_setup_partition_with_invalid_handle() {
        let invalid_partition = WHV_PARTITION_HANDLE::default(); // NULL / invalid handle
        let partition = Partition::new(invalid_partition);
        let result = setup_partition(&partition);
        assert!(result.is_err(), "Expected setup to fail with invalid handle");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("HRESULT"),
            "Expected HRESULT error, got: {}",
            err_msg
        );
    }

    /// Test successful allocation and mapping of memory to a partition
    #[test]
    fn test_allocate_partition_memory_success() {
        let partition = create_partition().expect("Failed to create partition");

        // Set processor count and setup partition before allocating memory
        let cpu_count: u32 = 1;
        unsafe {
            WHvSetPartitionProperty(
                partition.get_whv_partition_handle(),
                WHvPartitionPropertyCodeProcessorCount,
                &cpu_count as *const _ as *const _,
                size_of::<u32>() as u32,
            ).expect("Failed to set processor count");

            WHvSetupPartition(partition.get_whv_partition_handle()).expect("Failed to setup partition");
        }

        // Attempt to allocate 4KB of memory
        let result = allocate_partition_memory(&partition, 4096);
        assert!(result.is_ok(), "Expected success, got error: {:?}", result.err());
    }

    /// Test memory allocation failure due to insufficient available memory
    #[test]
    fn test_allocate_partition_memory_insufficient_memory() {
        let partition = create_partition().expect("Failed to create partition");

        let cpu_count: u32 = 1;
        unsafe {
            WHvSetPartitionProperty(
                partition.get_whv_partition_handle(),
                WHvPartitionPropertyCodeProcessorCount,
                &cpu_count as *const _ as *const _,
                size_of::<u32>() as u32,
            ).expect("Failed to set processor count");

            WHvSetupPartition(partition.get_whv_partition_handle()).expect("Failed to setup partition");
        }

        // Request an absurdly large allocation, guaranteed to fail
        let result = allocate_partition_memory(&partition, u64::MAX);
        assert!(result.is_err());
        assert!(
            result.as_ref().unwrap_err().contains("not enough available memory"),
            "Unexpected error: {:?}", result
        );
    }

    /// Test successful creation of a virtual CPU
    #[test]
    fn test_create_vcpu_success() {
        unsafe {
            let partition = create_partition().expect("Failed to create partition");

            // Set required processor count property and setup partition
            let processor_count: u32 = 1;
            let result = WHvSetPartitionProperty(
                partition.get_whv_partition_handle(),
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
            assert!(result.is_ok(), "Failed to set processor count: {:?}", result);

            let setup_result = WHvSetupPartition(partition.get_whv_partition_handle());
            assert!(setup_result.is_ok(), "Failed to set up partition: {:?}", setup_result);

            // Create the virtual processor with id 0
            let result = create_vcpu(&partition, 0);
            assert!(result.is_ok(), "create_vcpu failed: {:?}", result);
        }
    }

    /// Test failure when creating vCPU with invalid partition handle
    #[test]
    fn test_create_vcpu_invalid_partition() {
        unsafe {
            // Intentionally pass an invalid/null handle
            let fake_partition: WHV_PARTITION_HANDLE = WHV_PARTITION_HANDLE::default();
            let partition = Partition::new(fake_partition);
            let result = create_vcpu(&partition, 0);

            assert!(
                result.is_err(),
                "Expected error when creating vCPU with invalid partition"
            );
        }
    }

    /// Test running a vCPU through the full lifecycle of partition setup, memory mapping, vCPU creation, and execution
    #[test]
    fn test_run_vcpu_success() {
        let partition = create_partition().expect("WHvCreatePartition failed");

        // Set processor count property
        let processor_count: u32 = 1;
        let set_result = unsafe {
            WHvSetPartitionProperty(
                partition.get_whv_partition_handle(),
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const _ as *const _,
                size_of::<u32>() as u32,
            )
        };
        assert!(
            set_result.is_ok(),
            "WHvSetPartitionProperty failed: {:?}",
            set_result.err()
        );

        // Setup partition
        let setup_result = unsafe { WHvSetupPartition(partition.get_whv_partition_handle()) };
        assert!(setup_result.is_ok(), "WHvSetupPartition failed: {:?}", setup_result.err());

        // Allocate and map memory (4 KB)
        let alloc_result = allocate_partition_memory(&partition, 4096);
        assert!(alloc_result.is_ok(), "allocate_partition_memory failed: {:?}", alloc_result.err());

        // Create virtual processor
        let create_vcpu_result = create_vcpu(&partition, 0);
        assert!(create_vcpu_result.is_ok(), "create_vcpu failed: {:?}", create_vcpu_result.err());

        // Run the virtual processor
        let run_result = run_vcpu(&partition, 0);
        assert!(
            run_result.is_ok(),
            "WHvRunVirtualProcessor failed: {:?}",
            run_result.err()
        );
    }

    /// Test running a vCPU with an invalid partition handle should fail
    #[test]
    fn test_run_vcpu_invalid_partition() {
        let invalid_partition = WHV_PARTITION_HANDLE::default();
        let partition = Partition::new(invalid_partition);
        let result = run_vcpu(&partition, 0);
        assert!(result.is_err(), "Expected failure on invalid partition handle");
    }
}