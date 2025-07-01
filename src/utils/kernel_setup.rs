use tempfile::TempDir;
use std::fs::{read, create_dir, read_dir};
use std::process::Command;

#[derive(Debug)]
pub struct KernelComponents {
    pub kernel: Vec<u8>,
    pub initrd: Option<Vec<u8>>
}

pub fn extract_kernel_components_from_qcow2(qcow2_path: &str) -> Result<KernelComponents, String> {
    let temp_dir = match TempDir::new() {
        Ok(d) => d,
        Err(_) => return Err(format!("failed during temp_dir creation"))
    };

    let mount_dir = temp_dir.path().join("mount");
    if let Err(e) = create_dir(&mount_dir) {
        return Err(format!("{:?}", e));
    };
    let mount_str = match mount_dir.to_str() {
        Some(s) => s,
        None => return Err("failed during converting mount of type DirEntry to &str".to_string())
    };

    let auto_mount_exit_status = match Command::new("guestmount")
        .args(&["-a", qcow2_path, "-i", mount_str])
        .output() {
        Ok(s) => s,
        Err(e) => return Err(format!("{:?}", e))
    };

    if !auto_mount_exit_status.status.success() {
        return Err(format!(
            "guestmount failed with stderr: {}",
            String::from_utf8_lossy(&auto_mount_exit_status.stderr)
        ));
    }

    let boot_dir = mount_dir.as_path().join("boot");
    let path_to_boot_dir = match boot_dir.as_path().to_str() {
        Some(p) => p,
        None => return Err("failed during accessing boot directory".to_string())
    };

    let boot_entries = match read_dir(path_to_boot_dir) {
        Ok(e) => e,
        Err(e) => return Err(format!("failed during fetching entries conatined in boot directory"))
    };

    let mut path_to_vmlinuz_file: Option<String> = None;
    let mut path_to_initrd_file: Option<String> = None;

    for entry_res in boot_entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(e) => return Err(format!("{:?}", e)),
        };
        let filename = entry.file_name().to_string_lossy().into_owned();
        if filename.starts_with("vmlinuz") {
            path_to_vmlinuz_file = match entry.path().to_str() {
                Some(p) => Some(p.to_owned()),
                None => return Err(format!("failed to convert path to string slice"))
            };
        }
        else if filename.starts_with("initrd.img") {
            path_to_initrd_file = match entry.path().to_str() {
                Some(p) => Some(p.to_owned()),
                None => return Err(format!("failed to convert path to string slice"))
            };
        }
    }

    let path_to_vmlinuz_file = match path_to_vmlinuz_file {
        Some(p) => p,
        None => return Err(format!("vmlinuz file not found in boot directory"))
    };

    let vmlinuz_file_bytes: Vec<u8> = match read(path_to_vmlinuz_file) {
        Ok(b) => b,
        Err(e) => return Err(format!("{:?}", e))
    };

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