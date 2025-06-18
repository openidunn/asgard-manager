use AsgardManager::vm_setup::setup_utils::VmSetup;
use AsgardManager::vm_setup::linux_setup::run_vm;
use std::sync::Mutex;

// Constants for test setup

const TEST_MEM_1GB_MB: u32 = 1024;
const TEST_MEM_4GB_MB: u32 = 4096;
const TEST_MEM_1TB_MB: u32 = 1_048_576;
const TEST_MEM_MIN_MB: u32 = 1;

const TEST_CPU_1: u32 = 1;
const TEST_CPU_2: u32 = 2;
const TEST_CPU_32: u32 = 32;
const TEST_CPU_INVALID: u32 = 0;

// Prevent tests from colliding
static VM_TEST_LOCK: Mutex<()> = Mutex::new(());

// Error Checking Helpers

fn assert_vm_creation_error(e: &str) {
    assert!(
        e.contains("Failed to create KVM instance") ||
        e.contains("Failed to create VM"),
        "Expected VM creation error, got: {}", e
    );
}

fn assert_guest_memory_error(e: &str) {
    assert!(
        e.contains("Failed to create guest memory") ||
        e.contains("Failed to get host address"),
        "Expected guest memory setup error, got: {}", e
    );
}

fn assert_memory_region_error(e: &str) {
    assert!(
        e.contains("Failed to set memory region"),
        "Expected memory region setup error, got: {}", e
    );
}

fn assert_vcpu_creation_error(e: &str) {
    assert!(
        e.contains("Failed to create VCPU"),
        "Expected VCPU creation error, got: {}", e
    );
}

fn assert_vcpu_exit_or_runtime_error(e: &str) {
    assert!(
        e.contains("encountered an error") ||
        e.contains("Unhandled VCPU exit reason") ||
        e.contains("encountered IO") ||
        e.contains("MMIO") ||
        e.contains("Shutdown") ||
        e.contains("Task join error"),
        "Expected VCPU or runtime error, got: {}", e
    );
}

// Test Helpers

fn make_vmsetup(mem_mb: u32, cpus: u32) -> VmSetup {
    VmSetup::new(mem_mb, cpus)
}

// Actual Tests

#[tokio::test]
async fn test_run_vm_success_or_expected_error() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_1GB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran successfully with 1 GB, 1 CPU"),
        Err(e) => {
            assert_vm_creation_error(&e);
            assert_guest_memory_error(&e);
            assert_memory_region_error(&e);
            assert_vcpu_creation_error(&e);
            assert_vcpu_exit_or_runtime_error(&e);
        }
    }
}

#[tokio::test]
async fn test_run_vm_multiple_cpus() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_1GB_MB, TEST_CPU_2);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran with multiple CPUs"),
        Err(e) => {
            assert_vcpu_creation_error(&e);
            assert_vcpu_exit_or_runtime_error(&e);
        }
    }
}

#[tokio::test]
async fn test_run_vm_large_memory() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_4GB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran with 4 GB memory"),
        Err(e) => {
            assert_guest_memory_error(&e);
            assert_memory_region_error(&e);
            assert_vcpu_exit_or_runtime_error(&e);
        }
    }
}

#[tokio::test]
async fn test_run_vm_tremendous_memory() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_1TB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran with 1 TB memory"),
        Err(e) => {
            assert!(
                e.contains("Failed to create guest memory") ||
                e.contains("Failed to set memory region") ||
                e.contains("address space") ||
                e.contains("ENOMEM") ||
                e.contains("mmap"),
                "Expected large memory error, got: {}", e
            );
        }
    }
}

#[tokio::test]
async fn test_run_vm_minimal_memory() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_MIN_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran with minimal memory"),
        Err(e) => {
            assert_guest_memory_error(&e);
        }
    }
}

#[tokio::test]
async fn test_run_vm_many_cpus() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_4GB_MB, TEST_CPU_32);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran with 32 CPUs"),
        Err(e) => {
            assert_vcpu_creation_error(&e);
        }
    }
}

#[tokio::test]
async fn test_run_vm_massive_config() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_1TB_MB, TEST_CPU_32);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true, "VM ran with massive config (1TB, 32 CPUs)"),
        Err(e) => {
            assert!(
                e.contains("Failed to create guest memory") ||
                e.contains("Failed to create VCPU") ||
                e.contains("ENOMEM") ||
                e.contains("mmap") ||
                e.contains("Task join error"),
                "Expected resource exhaustion or VCPU error: {}", e
            );
        }
    }
}

#[tokio::test]
async fn test_run_vm_zero_cpus_should_fail() {
    let _mut_guard = VM_TEST_LOCK.lock().unwrap();
    let setup = make_vmsetup(TEST_MEM_1GB_MB, TEST_CPU_INVALID);
    let result = run_vm(setup).await;

    assert!(result.is_err(), "VM should not run with 0 CPUs");
    if let Err(e) = result {
        assert!(
            e.contains("Failed to create VCPU 0") ||
            e.contains("CPU count must be greater than zero") ||
            e.contains("invalid") ||
            e.contains("index out of bounds"),
            "Expected zero CPU rejection: {}", e
        );
    }
}