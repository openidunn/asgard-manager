use std::path::Path;
use AsgardManager::utils::kernel_setup::extract_kernel_components_from_qcow2;

#[test]
fn test_extract_kernel_invalid_path() {
    // Path doesn't exist - expect error
    let result = extract_kernel_components_from_qcow2("/nonexistent/path.qcow2");
    assert!(result.is_err());
    
    // Confirm error message contains relevant hint
    let err = result.unwrap_err();
    assert!(
        err.contains("guestmount failed") || err.contains("No such file"),
        "Unexpected error message: {}",
        err
    );
}

#[test]
fn test_extract_kernel_valid_image() {
    // Place a small valid qcow2 test image in tests/data/test_image.qcow2
    let test_image_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("test_image.qcow2");
    
    if !test_image_path.exists() {
        eprintln!("Skipping test_extract_kernel_valid_image: test image missing");
        return;
    }

    let result = extract_kernel_components_from_qcow2(test_image_path.to_str().unwrap());
    assert!(result.is_ok());

    let components = result.unwrap();
    assert!(!components.kernel.is_empty());
}

#[test]
fn test_extract_kernel_without_initrd() {
    let test_image_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("test_image_no_initrd.qcow2");

    if !test_image_path.exists() {
        eprintln!("Skipping test_extract_kernel_without_initrd: image missing");
        return;
    }

    let result = extract_kernel_components_from_qcow2(test_image_path.to_str().unwrap());
    assert!(result.is_ok());

    let components = result.unwrap();
    assert!(!components.kernel.is_empty());
    assert!(components.initrd.is_none());
}