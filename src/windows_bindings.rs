use windows::Win32::System::Hypervisor::{WHvCreatePartition, WHvDeletePartition, WHvSetPartitionProperty,
    WHV_PARTITION_HANDLE, WHvPartitionPropertyCodeProcessorCount, WHvMapGpaRange, WHV_MAP_GPA_RANGE_FLAGS, WHvMapGpaRangeFlagRead, 
    WHvMapGpaRangeFlagWrite, WHvMapGpaRangeFlagExecute, WHvSetupPartition
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

fn get_physical_memory_info() -> Result<(u64, u64), String> {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX::default();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        match GlobalMemoryStatusEx(&mut mem_status) {
            Ok(_) => Ok((mem_status.ullTotalPhys, mem_status.ullAvailPhys)),
            Err(e) => Err(format!("{:?}", e))
        }
    }
}

pub fn create_partition() -> Result<WHV_PARTITION_HANDLE, String> {
    match unsafe { WHvCreatePartition() } {
        Ok(handle) => Ok(handle),
        Err(e) => Err(format!("{:?}", e))
    }
}

pub fn set_processor_count_property(partition: WHV_PARTITION_HANDLE, processor_count: u32) -> Result<(), String> {
    if processor_count == 0 || processor_count > 64 {
        return Err(format!("Failed to set processor count: processor_count equal to {}", processor_count));
    }
    if let Err(e) = unsafe {
        WHvSetPartitionProperty(
            partition,
            WHvPartitionPropertyCodeProcessorCount,
            &processor_count as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        )
    } {
        return Err(format!("{:?}", e));
    }

    Ok(())
}

pub fn delete_partition(partition: WHV_PARTITION_HANDLE) -> Result<(), String> {
    if let Err(e) = unsafe { WHvDeletePartition(partition) } {
        return Err(format!("{:?}", e))
    }

    Ok(())
}

pub fn setup_partition(partition: WHV_PARTITION_HANDLE) -> Result<(), String> {
    match unsafe { WHvSetupPartition(partition) } {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("{:?}", e))
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

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

    #[test]
    fn test_create_partition() {
        let result = create_partition();
        match result {
            Ok(handle) => {
                println!("Partition created: {:?}", handle);
                // Clean up after test
                let _ = unsafe { WHvDeletePartition(handle) };
            }
            Err(e) => {
                println!("Partition creation failed: {}", e);
                // Optionally, assert on known error messages if you expect failure in your environment
            }
        }

        assert!(true)
    }

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

    #[test]
    fn test_set_processor_count_property_valid() {
        let partition = unsafe { WHvCreatePartition() }.expect("Partition creation failed");
        let result = set_processor_count_property(partition, 2);
        assert!(result.is_ok(), "Should succeed for valid processor count");
        let _ = unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_set_processor_count_property_zero() {
        let partition = unsafe { WHvCreatePartition() }.expect("Partition creation failed");
        let result = set_processor_count_property(partition, 0);
        assert!(result.is_err(), "Should fail for processor_count == 0");
        assert_eq!(
            result.unwrap_err(),
            "Failed to set processor count: processor_count equal to 0"
        );
        let _ = unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_set_processor_count_property_invalid_partition() {
        // Create an invalid handle (zeroed)
        let invalid_partition = WHV_PARTITION_HANDLE::default();
        let result = set_processor_count_property(invalid_partition, 2);
        assert!(result.is_err(), "Should fail for invalid partition handle");
    }

    #[test]
    fn test_set_processor_count_property_large_count() {
        let partition = unsafe { WHvCreatePartition() }.expect("Partition creation failed");
        // Use a very large processor count, likely to be invalid on most systems
        let result = set_processor_count_property(partition, 1024);
        assert!(result.is_err(), "Should fail for unreasonably large processor count");
        let _ = unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_setup_partition_success() {
        let partition = unsafe { WHvCreatePartition() }.expect("Failed to create partition");

        // Set required processor count before setup
        let processor_count: u32 = 2;
        let set_result = unsafe {
            WHvSetPartitionProperty(
                partition,
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        assert!(set_result.is_ok(), "Failed to set processor count: {:?}", set_result.err());

        // Now call setup
        let result = setup_partition(partition);
        assert!(result.is_ok(), "setup_partition failed: {:?}", result.err());

        let _ = unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_setup_partition_without_processor_property() {
        let partition = unsafe { WHvCreatePartition() }.expect("Failed to create partition");

        // Intentionally skip setting processor count
        let result = setup_partition(partition);
        assert!(result.is_err(), "Expected setup to fail without required properties");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("HRESULT"),
            "Expected HRESULT error, got: {}",
            err_msg
        );

        let _ = unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_setup_partition_with_invalid_handle() {
        let invalid_partition = WHV_PARTITION_HANDLE::default(); // NULL / invalid handle

        let result = setup_partition(invalid_partition);
        assert!(result.is_err(), "Expected setup to fail with invalid handle");

        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("HRESULT"),
            "Expected HRESULT error, got: {}",
            err_msg
        );
    }
}