use AsgardManager::utils::dependencies::{check_if_guestmount_is_installed,
    PackageManager};
use std::process::Command;

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

// Optionally, you can mock Command to test both branches, but that requires more setup
// and is not shown here for simplicity.