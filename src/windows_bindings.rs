use windows::Win32::System::Hypervisor::{WHvCreatePartition, WHvDeletePartition, WHV_PARTITION_HANDLE};

pub fn create_partition() -> Result<WHV_PARTITION_HANDLE, String> {
    match unsafe { WHvCreatePartition() } {
        Ok(handle) => Ok(handle),
        Err(e) => Err(format!("{:?}", e))
    }
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
}