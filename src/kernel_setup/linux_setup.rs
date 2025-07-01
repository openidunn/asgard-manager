use tempfile::TempDir;
use std::fs::{read, create_dir, read_dir};
use std::process::Command;
use super::setup_utils::KernelComponents;

/// Extracts kernel components (vmlinuz and optionally initrd) from a QCOW2 disk image.
///
/// This function uses `guestmount` to mount the QCOW2 image and then looks for kernel
/// files under the `/boot` directory. It returns a `KernelComponents` struct containing
/// the raw bytes of the kernel and optionally the initrd image.
///
/// # Arguments
/// * `qcow2_path` - Path to the `.qcow2` disk image file.
///
/// # Returns
/// * `Ok(KernelComponents)` - On success, contains the loaded kernel and optionally initrd.
/// * `Err(String)` - If any step fails, returns a descriptive error message.
pub fn extract_kernel_components_from_qcow2(qcow2_path: &str) -> Result<KernelComponents, String> {
    // Create a temporary directory to mount the image
    let temp_dir = match TempDir::new() {
        Ok(d) => d,
        Err(_) => return Err(format!("failed during temp_dir creation"))
    };

    // Create a mount point inside the temp directory
    let mount_dir = temp_dir.path().join("mount");
    if let Err(e) = create_dir(&mount_dir) {
        return Err(format!("{:?}", e));
    };

    // Convert mount path to a string slice
    let mount_str = match mount_dir.to_str() {
        Some(s) => s,
        None => return Err("failed during converting mount of type DirEntry to &str".to_string())
    };

    // Use guestmount to mount the qcow2 image at the mount point
    let auto_mount_exit_status = match Command::new("guestmount")
        .args(&["-a", qcow2_path, "-i", mount_str])
        .output() {
        Ok(s) => s,
        Err(e) => return Err(format!("{:?}", e))
    };

    // Check if guestmount succeeded
    if !auto_mount_exit_status.status.success() {
        return Err(format!(
            "guestmount failed with stderr: {}",
            String::from_utf8_lossy(&auto_mount_exit_status.stderr)
        ));
    }

    // Construct path to the /boot directory inside the mounted image
    let boot_dir = mount_dir.as_path().join("boot");
    let path_to_boot_dir = match boot_dir.as_path().to_str() {
        Some(p) => p,
        None => return Err("failed during accessing boot directory".to_string())
    };

    // Read entries inside /boot to locate kernel and initrd files
    let boot_entries = match read_dir(path_to_boot_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("failed during fetching entries conatined in boot directory"))
    };

    let mut path_to_vmlinuz_file: Option<String> = None;
    let mut path_to_initrd_file: Option<String> = None;

    // Loop over all files in /boot to find vmlinuz and initrd.img
    for entry_res in boot_entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(e) => return Err(format!("{:?}", e)),
        };
        let filename = entry.file_name().to_string_lossy().into_owned();
        if filename.starts_with("vmlinuz") {
            // Found kernel image
            path_to_vmlinuz_file = match entry.path().to_str() {
                Some(p) => Some(p.to_owned()),
                None => return Err(format!("failed to convert path to string slice"))
            };
        }
        else if filename.starts_with("initrd.img") {
            // Found initrd image
            path_to_initrd_file = match entry.path().to_str() {
                Some(p) => Some(p.to_owned()),
                None => return Err(format!("failed to convert path to string slice"))
            };
        }
    }

    // If kernel image is missing, fail
    let path_to_vmlinuz_file = match path_to_vmlinuz_file {
        Some(p) => p,
        None => return Err(format!("vmlinuz file not found in boot directory"))
    };

    // Read kernel file into memory
    let vmlinuz_file_bytes: Vec<u8> = match read(path_to_vmlinuz_file) {
        Ok(b) => b,
        Err(e) => return Err(format!("{:?}", e))
    };

    // Read initrd file if present and return both as KernelComponents
    match path_to_initrd_file {
        Some(p) => {
            let initrd_file_bytes: Vec<u8> = match read(p) {
                Ok(b) => b,
                Err(e) => return Err(format!("{:?}", e))
            };
            Ok(KernelComponents {kernel: vmlinuz_file_bytes, initrd: Some(initrd_file_bytes)})
        },
        None => Ok(KernelComponents {kernel: vmlinuz_file_bytes, initrd: None})
    }
}