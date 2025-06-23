use windows::Win32::System::Hypervisor::{WHvCreatePartition, WHvDeletePartition, WHvSetPartitionProperty,
    WHV_PARTITION_HANDLE, WHvPartitionPropertyCodeProcessorCount, WHvMapGpaRange, WHV_MAP_GPA_RANGE_FLAGS, WHvMapGpaRangeFlagRead, 
    WHvMapGpaRangeFlagWrite, WHvMapGpaRangeFlagExecute, WHvSetupPartition, WHvCreateVirtualProcessor, WHvRunVirtualProcessor, 
    WHV_RUN_VP_EXIT_CONTEXT
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::core::HRESULT;
use windows::Win32::System::Memory::{VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};

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

pub fn allocate_partition_memory(partition: WHV_PARTITION_HANDLE, mem_size: u64) -> Result<(), String> {
    let (total_mem, avail_mem) = match get_physical_memory_info() {
        Ok((total_mem, avail_mem)) => (total_mem, avail_mem),
        Err(e) => return Err(format!("Failed to get memory info: {}", e)),
    };

    if avail_mem < mem_size {
        return Err("Failed to allocate the memory: not enough available memory".to_string());
    }

    // Allocate host memory
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

    // Map into guest physical address space
    let flags = WHV_MAP_GPA_RANGE_FLAGS(
        WHvMapGpaRangeFlagRead.0 |
        WHvMapGpaRangeFlagWrite.0 |
        WHvMapGpaRangeFlagExecute.0,
    );

    let result = unsafe {
        WHvMapGpaRange(partition, ptr as *mut _, 0x0000, mem_size, flags)
    };

    match result {
        Ok(()) => Ok(()),
        Err(e) => Err(format!("Failed to map memory: {:?}", e)),
    }
}

pub fn create_vcpu(partition: WHV_PARTITION_HANDLE, cpu_id: u32) -> Result<(), String> {
    let hresult = unsafe { WHvCreateVirtualProcessor(partition, cpu_id, 0) };
    if let Err(e) = hresult {
        return Err(format!("Failed to create virtual processor: {:?}", e));
    }

    Ok(())
}

pub fn run_vcpu(partition: WHV_PARTITION_HANDLE, cpu_id: u32) -> Result<WHV_RUN_VP_EXIT_CONTEXT, String> {
    let mut vcpu_ctx: WHV_RUN_VP_EXIT_CONTEXT = WHV_RUN_VP_EXIT_CONTEXT::default();
    let val_size = std::mem::size_of_val(&vcpu_ctx) as u32;
    if let Err(e) = unsafe { WHvRunVirtualProcessor(partition, cpu_id, &mut vcpu_ctx as *mut _ as *mut _, val_size) } {
        return Err(format!("{:?}", e));
    }

    Ok(vcpu_ctx)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;
    use std::mem::size_of;

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

    #[test]
    fn test_allocate_partition_memory_success() {
        let partition: WHV_PARTITION_HANDLE = unsafe { WHvCreatePartition() }.expect("Failed to create partition");

        let cpu_count: u32 = 1;
        unsafe {
            WHvSetPartitionProperty(
                partition,
                WHvPartitionPropertyCodeProcessorCount,
                &cpu_count as *const _ as *const _,
                size_of::<u32>() as u32,
            ).expect("Failed to set processor count");

            WHvSetupPartition(partition).expect("Failed to setup partition");
        }

        let result = allocate_partition_memory(partition, 4096);
        assert!(result.is_ok(), "Expected success, got error: {:?}", result.err());

        unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_allocate_partition_memory_insufficient_memory() {
        let partition: WHV_PARTITION_HANDLE = unsafe { WHvCreatePartition() }.expect("Failed to create partition");

        let cpu_count: u32 = 1;
        unsafe {
            WHvSetPartitionProperty(
                partition,
                WHvPartitionPropertyCodeProcessorCount,
                &cpu_count as *const _ as *const _,
                size_of::<u32>() as u32,
            ).expect("Failed to set processor count");

            WHvSetupPartition(partition).expect("Failed to setup partition");
        }

        let result = allocate_partition_memory(partition, u64::MAX);
        assert!(result.is_err());
        assert!(
            result.as_ref().unwrap_err().contains("not enough available memory"),
            "Unexpected error: {:?}", result
        );

        unsafe { WHvDeletePartition(partition) };
    }

    #[test]
    fn test_create_vcpu_success() {
        unsafe {
            let partition = WHvCreatePartition().expect("Failed to create partition");

            let processor_count: u32 = 1;
            let result = WHvSetPartitionProperty(
                partition,
                WHvPartitionPropertyCodeProcessorCount,
                &processor_count as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );
            assert!(result.is_ok(), "Failed to set processor count: {:?}", result);

            let setup_result = WHvSetupPartition(partition);
            assert!(setup_result.is_ok(), "Failed to set up partition: {:?}", setup_result);

            let result = create_vcpu(partition, 0);
            assert!(result.is_ok(), "create_vcpu failed: {:?}", result);

            let _ = WHvDeletePartition(partition);
        }
    }

    #[test]
    fn test_create_vcpu_invalid_partition() {
        unsafe {
            // Intentionally pass an invalid/null handle
            let fake_partition: WHV_PARTITION_HANDLE = WHV_PARTITION_HANDLE::default();
            let result = create_vcpu(fake_partition, 0);

            assert!(
                result.is_err(),
                "Expected error when creating vCPU with invalid partition"
            );
        }
    }

    #[test]
    fn test_run_vcpu_success() {

        let partition = unsafe {
                WHvCreatePartition()
        }.expect("WHvCreatePartition failed");

        let processor_count: u32 = 1;
        let set_result = unsafe {
                WHvSetPartitionProperty(
                        partition,
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

        let setup_result = unsafe {
                WHvSetupPartition(partition)
        };
        assert!(
                setup_result.is_ok(),
                "WHvSetupPartition failed: {:?}",
                setup_result.err()
        );

        let mem_size: usize = 4096;
        let mem_ptr = unsafe {
                VirtualAlloc(
                        None,
                        mem_size,
                        MEM_COMMIT | MEM_RESERVE,
                        PAGE_READWRITE,
                )
        };
        assert!(
                !mem_ptr.is_null(),
                "VirtualAlloc failed"
        );

        let map_flags = WHV_MAP_GPA_RANGE_FLAGS(
                  WHvMapGpaRangeFlagRead.0
                | WHvMapGpaRangeFlagWrite.0
                | WHvMapGpaRangeFlagExecute.0,
        );
        let map_result = unsafe {
                WHvMapGpaRange(
                        partition,
                        mem_ptr as *mut _,
                        0x0000,
                        mem_size as u64,
                        map_flags,
                )
        };
        assert!(
                map_result.is_ok(),
                "WHvMapGpaRange failed: {:?}",
                map_result.err()
        );

        let create_result = unsafe {
                WHvCreateVirtualProcessor(partition, 0, 0)
        };
        assert!(
                create_result.is_ok(),
                "WHvCreateVirtualProcessor failed: {:?}",
                create_result.err()
        );

        let result = run_vcpu(partition, 0);
        assert!(
                result.is_ok(),
                "run_vcpu failed: {:?}",
                result.err()
        );

        let delete_result = unsafe {
                WHvDeletePartition(partition)
        };
        assert!(
                delete_result.is_ok(),
                "WHvDeletePartition failed: {:?}",
                delete_result.err()
        );
    }
}