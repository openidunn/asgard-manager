use AsgardManager::vm_setup::setup_utils::VmSetup;
use std::sync::Mutex;

const TEST_MB: u32 = 4;
const TEST_CPU_CORES: u32 = 2;
const ZERO_MB: u32 = 0;
const ZERO_CORES: u32 = 0;
const ONE_CORE: u32 = 1;

static VM_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_vmsetup_new_with_valid_values() {
    let setup = VmSetup::new(TEST_MB, TEST_CPU_CORES);
    assert_eq!(setup.get_memory_size(), (1024 * 1024 * TEST_MB) as usize);
    assert_eq!(setup.get_cpu_cores_count(), TEST_CPU_CORES);
}

#[test]
fn test_vmsetup_new_with_zero_cores_sets_two() {
    let setup = VmSetup::new(TEST_MB, ZERO_CORES);
    assert_eq!(setup.get_cpu_cores_count(), 2);
}

#[test]
fn test_vmsetup_new_with_one_core_sets_two() {
    let setup = VmSetup::new(TEST_MB, ONE_CORE);
    assert_eq!(setup.get_cpu_cores_count(), 2);
}

#[test]
fn test_vmsetup_new_with_zero_memory() {
    let setup = VmSetup::new(ZERO_MB, TEST_CPU_CORES);
    assert_eq!(setup.get_memory_size(), 0);
    assert_eq!(setup.get_cpu_cores_count(), TEST_CPU_CORES);
}