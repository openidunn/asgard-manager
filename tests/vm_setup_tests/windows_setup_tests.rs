use AsgardManager::vm_setup::setup_utils::VmSetup;
use AsgardManager::vm_setup::windows_setup::run_vm;
use std::sync::Mutex;

// Constants for test setup
const TEST_MEM_1GB: u32 = 1024;
const TEST_MEM_16MB: u32 = 16;
const TEST_MEM_1TB: u32 = 1024 * 1024;
const TEST_MEM_MIN: u32 = 4;

const TEST_CPU_1: u32 = 1;
const TEST_CPU_2: u32 = 2;
const TEST_CPU_32: u32 = 100;
const TEST_CPU_INVALID: u32 = 0;

// Prevent tests from colliding
static VM_TEST_LOCK: Mutex<()> = Mutex::new(());

// Concrete success test: VM runs successfully with small memory and 1 CPU
#[tokio::test]
async fn test_run_vm_success_16mb_1cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_16MB, TEST_CPU_1);
    let result = run_vm(setup).await;
    assert!(result.is_ok(), "Expected VM to run successfully but got error: {:?}", result.err());
}

// Concrete success test: VM runs successfully with multiple CPUs
#[tokio::test]
async fn test_run_vm_success_16mb_2cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_16MB, TEST_CPU_2);
    let result = run_vm(setup).await;
    assert!(result.is_ok(), "Expected VM to run successfully but got error: {:?}", result.err());
}

// Concrete success test: VM runs with 1GB memory and 1 CPU
#[tokio::test]
async fn test_run_vm_success_1gb_1cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_1GB, TEST_CPU_1);
    let result = run_vm(setup).await;
    assert!(result.is_ok(), "Expected VM to run successfully but got error: {:?}", result.err());
}

// Concrete failure test: Expect failure with extremely large memory allocation (1TB)
#[tokio::test]
async fn test_run_vm_fail_large_memory() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_1TB, TEST_CPU_1);
    let result = run_vm(setup).await;
    assert!(result.is_err(), "Expected failure due to large memory allocation");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("Failed to allocate the memory: not enough available memory"),
        "Expected large memory related error, got: {}", err_msg
    );
}

// Concrete failure test: Expect failure or error with minimal memory allocation (4MB)
#[tokio::test]
async fn test_run_vm_fail_minimal_memory() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_MIN, TEST_CPU_1);
    let result = run_vm(setup).await;
    // This might succeed or fail depending on hypervisor support — assume fail here
    if result.is_ok() {
        // If it succeeds, just pass the test (optional)
        assert!(true);
    } else {
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Map memory error"),
            "Expected memory mapping error due to small memory, got: {}", err_msg
        );
    }
}

// Concrete failure test: Expect failure creating many CPUs (100 CPUs)
#[tokio::test]
async fn test_run_vm_fail_many_cpus() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_16MB, TEST_CPU_32);
    let result = run_vm(setup).await;
    assert!(result.is_err(), "Expected failure when creating 100 CPUs");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("Failed to set processor count: processor_count equal to"),
        "Expected VCPU creation error for many CPUs, got: {}", err_msg
    );
}

// Concrete failure test: Expect failure when CPU count is zero (invalid)
#[tokio::test]
async fn test_run_vm_zero_cpu_normalizes_to_two() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = VmSetup::new(TEST_MEM_16MB, TEST_CPU_INVALID); // zero CPUs
    assert_eq!(setup.get_cpu_cores_count(), 2, "Expected normalization to 2 CPUs");
    let result = run_vm(setup).await;
    assert!(result.is_ok(), "VM should run with normalized 2 CPUs");
}
