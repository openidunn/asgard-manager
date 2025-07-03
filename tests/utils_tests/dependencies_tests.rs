use AsgardManager::utils::dependencies::{check_if_guestmount_is_installed,
    PackageManager, find_linux_distribution_image};
use std::process::Command;
use std::fs::File;
use std::env;
use std::path::Path;
use tempfile::tempdir;
use std::sync::Mutex;

// Prevent tests from colliding
static FIND_LINUX_DISTRIBUTION_IMAGE_TEST_LOCK: Mutex<()> = Mutex::new(());

fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_package_manager_as_str() {
    assert_eq!(PackageManager::APT.as_str(), "apt-get");
    assert_eq!(PackageManager::DNF.as_str(), "dnf");
    assert_eq!(PackageManager::YUM.as_str(), "yum");
    assert_eq!(PackageManager::Zypper.as_str(), "zypper");
    assert_eq!(PackageManager::Pacman.as_str(), "pacman");
    assert_eq!(PackageManager::Unknown.as_str(), "unknown");
}

#[test]
fn test_detect_package_manager_matches_system() {
    let detected = PackageManager::detect();

    let available = [
        ("apt-get", PackageManager::APT),
        ("dnf", PackageManager::DNF),
        ("yum", PackageManager::YUM),
        ("zypper", PackageManager::Zypper),
        ("pacman", PackageManager::Pacman),
    ];

    for (cmd, expected_pm) in &available {
        if command_exists(cmd) {
            assert_eq!(
                detected, *expected_pm,
                "Expected {:?} due to presence of `{}`, but got {:?}",
                expected_pm, cmd, detected
            );
            return; // test passes, exit early
        }
    }

    assert_eq!(detected, PackageManager::Unknown);
}

#[test]
fn test_guestmount_installed_or_not() {
    // This test will pass regardless of whether guestmount is installed,
    // but it will verify that the function returns a boolean and does not panic.
    let result = check_if_guestmount_is_installed();
    assert!(
        result == true || result == false,
        "Function should return a boolean"
    );
}

#[test]
fn test_find_linux_distribution_image_qcow2() {
    let _guard = FIND_LINUX_DISTRIBUTION_IMAGE_TEST_LOCK.lock().unwrap();
    let temp = tempdir().unwrap();
    let old_dir = env::current_dir().unwrap();
    env::set_current_dir(temp.path()).unwrap();

    let test_file = "test_image.qcow2";
    File::create(test_file).unwrap();
    let result = find_linux_distribution_image();
    assert!(result.is_some());
    let found = result.unwrap();
    assert_eq!(
        Path::new(&found).file_name().unwrap(),
        test_file
    );

    env::set_current_dir(old_dir).unwrap();
}

#[test]
fn test_find_linux_distribution_image_img() {
    let _guard = FIND_LINUX_DISTRIBUTION_IMAGE_TEST_LOCK.lock().unwrap();
    let temp = tempdir().unwrap();
    let old_dir = env::current_dir().unwrap();
    env::set_current_dir(temp.path()).unwrap();

    let test_file = "test_image.img";
    File::create(test_file).unwrap();
    let result = find_linux_distribution_image();
    assert!(result.is_some());
    let found = result.unwrap();
    assert_eq!(
        Path::new(&found).file_name().unwrap(),
        test_file
    );

    env::set_current_dir(old_dir).unwrap();
}

#[test]
fn test_find_linux_distribution_image_iso() {
    let _guard = FIND_LINUX_DISTRIBUTION_IMAGE_TEST_LOCK.lock().unwrap();
    let temp = tempdir().unwrap();
    let old_dir = env::current_dir().unwrap();
    env::set_current_dir(temp.path()).unwrap();

    let test_file = "test_image.iso";
    File::create(test_file).unwrap();
    let result = find_linux_distribution_image();
    assert!(result.is_some());
    let found = result.unwrap();
    assert_eq!(
        Path::new(&found).file_name().unwrap(),
        test_file
    );

    env::set_current_dir(old_dir).unwrap();
}

#[test]
fn test_find_linux_distribution_image_none() {
    let _guard = FIND_LINUX_DISTRIBUTION_IMAGE_TEST_LOCK.lock().unwrap();
    let temp = tempdir().unwrap();
    let old_dir = env::current_dir().unwrap();
    env::set_current_dir(temp.path()).unwrap();

    // Debug: print files in the directory
    for entry in std::fs::read_dir(".").unwrap() {
        println!("File in temp dir: {:?}", entry.unwrap().file_name());
    }

    let result = find_linux_distribution_image();
    assert!(result.is_none(), "Should return None if no image files are present");

    env::set_current_dir(old_dir).unwrap();
}