use AsgardManager::vm_setup::setup_utils::VmSetup;
use AsgardManager::vm_setup::linux_setup::run_vm;
use std::sync::Mutex;

// Constants for test setup
const TEST_MEM_1GB_MB: u32 = 1024;
const TEST_MEM_2GB_MB: u32 = 2048;
const TEST_MEM_4GB_MB: u32 = 4096;
const TEST_MEM_1TB_MB: u32 = 1_048_576;
const TEST_MEM_MIN_MB: u32 = 4;

const TEST_CPU_1: u32 = 1;
const TEST_CPU_2: u32 = 2;
const TEST_CPU_4: u32 = 4;
const TEST_CPU_8: u32 = 8;
const TEST_CPU_32: u32 = 32;
const TEST_CPU_INVALID: u32 = 0;

// Prevent tests from colliding
static VM_TEST_LOCK: Mutex<()> = Mutex::new(());

// General-purpose error matchers
fn assert_vm_creation_error(e: &str) -> bool {
    e.contains("Failed to create KVM instance") || e.contains("Failed to create VM")
}
fn assert_guest_memory_error(e: &str) -> bool {
    e.contains("Failed to create guest memory") || e.contains("Failed to get host address")
}
fn assert_memory_region_error(e: &str) -> bool {
    e.contains("Failed to set memory region")
}
fn assert_vcpu_creation_error(e: &str) -> bool {
    e.contains("Failed to create VCPU")
}
fn assert_vcpu_exit_or_runtime_error(e: &str) -> bool {
    e.contains("encountered an error")
        || e.contains("Unhandled VCPU exit reason")
        || e.contains("encountered IO")
        || e.contains("MMIO")
        || e.contains("Shutdown")
        || e.contains("Task join error")
}

// Dedicated error expectations
fn assert_error_for_1gb_1cpu(e: &str) {
    assert!(
        assert_vm_creation_error(e)
            || assert_guest_memory_error(e)
            || assert_memory_region_error(e)
            || assert_vcpu_creation_error(e)
            || assert_vcpu_exit_or_runtime_error(e),
        "Unexpected error for 1GB/1CPU: {}", e
    );
}
fn assert_error_for_2cpu(e: &str) {
    assert!(
        assert_vcpu_creation_error(e) || assert_vcpu_exit_or_runtime_error(e),
        "Unexpected error for 1GB/2CPU: {}", e
    );
}
fn assert_error_for_4gb(e: &str) {
    assert!(
        assert_guest_memory_error(e)
            || assert_memory_region_error(e)
            || assert_vcpu_exit_or_runtime_error(e),
        "Unexpected error for 4GB memory config: {}", e
    );
}
fn assert_error_for_1tb(e: &str) {
    assert!(
        e.contains("Failed to create guest memory")
            || e.contains("Failed to set memory region")
            || e.contains("address space")
            || e.contains("ENOMEM")
            || e.contains("mmap"),
        "Unexpected error for 1TB memory: {}", e
    );
}
fn assert_error_for_min_memory(e: &str) {
    assert!(
        assert_guest_memory_error(e),
        "Unexpected error for minimal memory: {}", e
    );
}
fn assert_error_for_32cpus(e: &str) {
    assert!(
        assert_vcpu_creation_error(e),
        "Unexpected error for 32 CPUs: {}", e
    );
}
fn assert_error_for_massive_config(e: &str) {
    assert!(
        e.contains("Failed to set memory region"),
        "Unexpected error for 1TB/32CPU config: {}", e
    );
}
fn assert_error_for_zero_cpu(e: &str) {
    assert!(
        e.contains("Failed to create VCPU 0")
            || e.contains("CPU count must be greater than zero")
            || e.contains("invalid")
            || e.contains("index out of bounds"),
        "Unexpected error for 0 CPU: {}", e
    );
}

// Setup builder
fn make_vmsetup(mem_mb: u32, cpus: u32) -> VmSetup {
    VmSetup::new(mem_mb, cpus)
}

// Tests expecting success only

#[tokio::test]
async fn test_run_vm_ok_2gb_1cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_2GB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    assert!(result.is_ok(), "Expected VM to run with 2GB/1CPU, got error: {:?}", result);
}

#[tokio::test]
async fn test_run_vm_ok_4gb_2cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_4GB_MB, TEST_CPU_2);
    let result = run_vm(setup).await;

    assert!(result.is_ok(), "Expected VM to run with 4GB/2CPU, got error: {:?}", result);
}

#[tokio::test]
async fn test_run_vm_ok_2gb_4cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_2GB_MB, TEST_CPU_4);
    let result = run_vm(setup).await;

    assert!(result.is_ok(), "Expected VM to run with 2GB/4CPU, got error: {:?}", result);
}

#[tokio::test]
async fn test_run_vm_ok_4gb_8cpu() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_4GB_MB, TEST_CPU_8);
    let result = run_vm(setup).await;

    assert!(result.is_ok(), "Expected VM to run with 4GB/8CPU, got error: {:?}", result);
}

// Tests that expect success or known failure modes

#[tokio::test]
async fn test_run_vm_success_or_expected_error() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_1GB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_1gb_1cpu(&e),
    }
}

#[tokio::test]
async fn test_run_vm_multiple_cpus() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_1GB_MB, TEST_CPU_2);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_2cpu(&e),
    }
}

#[tokio::test]
async fn test_run_vm_large_memory() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_4GB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_4gb(&e),
    }
}

#[tokio::test]
async fn test_run_vm_tremendous_memory() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_1TB_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_1tb(&e),
    }
}

#[tokio::test]
async fn test_run_vm_minimal_memory() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_MIN_MB, TEST_CPU_1);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_min_memory(&e),
    }
}

#[tokio::test]
async fn test_run_vm_many_cpus() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_4GB_MB, TEST_CPU_32);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_32cpus(&e),
    }
}

#[tokio::test]
async fn test_run_vm_massive_config() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_1TB_MB, TEST_CPU_32);
    let result = run_vm(setup).await;

    match result {
        Ok(()) => assert!(true),
        Err(e) => assert_error_for_massive_config(&e),
    }
}

#[tokio::test]
async fn test_run_vm_zero_cpus_should_fail() {
    let _guard = VM_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let setup = make_vmsetup(TEST_MEM_1GB_MB, TEST_CPU_INVALID);
    let result = run_vm(setup).await;

    assert!(result.is_err(), "VM should not run with 0 CPUs");
    if let Err(e) = result {
        assert_error_for_zero_cpu(&e);
    }
}
