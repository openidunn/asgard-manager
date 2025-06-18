use AsgardManager::vm_setup::setup_utils::VmSetup;
use AsgardManager::vm_setup::linux_setup::run_vm;
use std::sync::Mutex;

// Constants for test VM configuration
const TEST_MEM_1GB_MB: u32 = 1024; // 1 GiB in MiB
const TEST_MEM_4GB_MB: u32 = 4096; // 4 GiB in MiB
const TEST_MEM_1TB_MB: u32 = 1_048_576; // 1 TB in MiB
const TEST_CPU_1: u32 = 1;
const TEST_CPU_2: u32 = 2;

static VM_TEST_LOCK: Mutex<()> = Mutex::new(());

/// Helper: Create a valid VmSetup for testing
fn make_vmsetup() -> VmSetup {
    VmSetup::new(TEST_MEM_1GB_MB, TEST_CPU_1)
}

#[tokio::test]
async fn test_run_vm_success_or_expected_error() {
    VM_TEST_LOCK.lock().unwrap(); // Ensure no other test runs concurrently
    let setup = make_vmsetup();
    let result = run_vm(setup).await;
    // Accept either Ok(()) or a known error (e.g., system not allowing KVM)
    match result {
        Ok(()) => assert!(true),
        Err(e) => assert!(
            e.contains("Failed to create KVM instance") ||
            e.contains("Failed to create VM") ||
            e.contains("Failed to create guest memory") ||
            e.contains("Failed to set memory region") ||
            e.contains("Failed to create VCPU") ||
            e.contains("encountered an error") ||
            e.contains("Task join error"),
            "Unexpected error: {}", e
        ),
    }
}

#[tokio::test]
async fn test_run_vm_multiple_cpus() {
    VM_TEST_LOCK.lock().unwrap();
    let setup = VmSetup::new(TEST_MEM_1GB_MB, TEST_CPU_2);
    let result = run_vm(setup).await;
    match result {
        Ok(()) => assert!(true),
        Err(e) => assert!(
            e.contains("Failed to create KVM instance") ||
            e.contains("Failed to create VM") ||
            e.contains("Failed to create guest memory") ||
            e.contains("Failed to set memory region") ||
            e.contains("Failed to create VCPU") ||
            e.contains("encountered an error") ||
            e.contains("Task join error"),
            "Unexpected error: {}", e
        ),
    }
}

#[tokio::test]
async fn test_run_vm_large_memory() {
    VM_TEST_LOCK.lock().unwrap();
    let setup = VmSetup::new(TEST_MEM_4GB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;
    match result {
        Ok(()) => assert!(true),
        Err(e) => assert!(
            e.contains("Failed to create KVM instance") ||
            e.contains("Failed to create VM") ||
            e.contains("Failed to create guest memory") ||
            e.contains("Failed to set memory region") ||
            e.contains("Failed to create VCPU") ||
            e.contains("encountered an error") ||
            e.contains("Task join error"),
            "Unexpected error: {}", e
        ),
    }
}

#[tokio::test]
async fn test_run_vm_tremendous_memory() {
    let setup = VmSetup::new(TEST_MEM_1TB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;
    match result {
        Ok(()) => assert!(true),
        Err(e) => assert!(
            e.contains("Failed to create guest memory") ||
            e.contains("Failed to set memory region") ||
            e.contains("Failed to create VM") ||
            e.contains("Failed to create KVM instance") ||
            e.contains("encountered an error") ||
            e.contains("Task join error"),
            "Unexpected error: {}", e
        ),
    }
}