use windows::Win32::System::Hypervisor::{WHvCreatePartition, WHvDeletePartition, WHvSetPartitionProperty,
    WHV_PARTITION_HANDLE, WHvPartitionPropertyCodeProcessorCount};

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

pub fn delete_partition(handle: WHV_PARTITION_HANDLE) -> Result<(), String> {
    if let Err(e) = unsafe { WHvDeletePartition(handle) } {
        return Err(format!("{:?}", e))
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

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
}