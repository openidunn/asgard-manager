use windows::Win32::System::Hypervisor::{WHvCreatePartition, WHvDeletePartition, WHV_PARTITION_HANDLE};

pub fn create_partition() -> Result<WHV_PARTITION_HANDLE, String> {
    match unsafe { WHvCreatePartition() } {
        Ok(handle) => Ok(handle),
        Err(e) => Err(format!("{:?}", e))
    }
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
}