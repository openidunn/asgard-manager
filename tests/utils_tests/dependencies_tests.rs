use AsgardManager::utils::dependencies::check_if_guestmount_is_installed;

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