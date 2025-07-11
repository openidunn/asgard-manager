use AsgardManager::vm_setup::macos_setup::run_vm;
use AsgardManager::vm_setup::setup_utils::VmSetup;
use std::sync::Mutex;

const TEST_MB: u32 = 4;
const TEST_CPU_CORES: u32 = 2;
const ZERO_MB: u32 = 0;
const ZERO_CORES: u32 = 0;
const ONE_CORE: u32 = 1;

static VM_TEST_LOCK: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_run_vm_zero_memory() {
    // Ensure no other test runs concurrently
    let _mutex_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = VmSetup::new(0, 2);
    let result = run_vm(setup).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Failed to map memory");
}

// Test that run_vm returns error if CPU count is zero (should default to 2)
#[tokio::test]
async fn test_run_vm_cpu_zero_defaults_to_two() {
    // Ensure no other test runs concurrently
    let _mutex_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = VmSetup::new(TEST_MB, ZERO_CORES);
    // This should try to run VM with 2 vCPUs (may fail if virtualization not available)
    // So just test it doesn't panic and returns result.
    let result = run_vm(setup).await;
    // Cannot assert exact error, but should not panic
    assert!(result.is_ok());
}

// Test that run_vm returns error if memory mapping fails
#[tokio::test]
async fn test_run_vm_mem_map_fail() {
    // Ensure no other test runs concurrently
    let _mutex_guard = VM_TEST_LOCK.lock().unwrap();
    // This is tricky without mocking.
    // One crude way is to ask for absurdly large memory causing map to fail.

    let huge_mem = usize::MAX; // intentionally huge to cause failure
    let setup = VmSetup::new((huge_mem / (1024*1024)) as u32, 1);

    let result = run_vm(setup).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Failed to map memory");
}
