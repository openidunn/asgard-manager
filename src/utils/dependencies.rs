use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum PackageManager {
    APT,
    DNF,
    YUM,
    Zypper,
    Pacman,
    Unknown
}

impl PackageManager {
    pub fn as_str(&self) -> &str {
        match self {
            PackageManager::APT => "apt-get",
            PackageManager::DNF => "dnf",
            PackageManager::YUM => "yum",
            PackageManager::Zypper => "zypper",
            PackageManager::Pacman => "pacman",
            PackageManager::Unknown => "unknown",
        }
    }
    pub fn detect() -> PackageManager {
        if Command::new("apt-get").arg("--version").output().is_ok() {
            PackageManager::APT
        } else if Command::new("dnf").arg("--version").output().is_ok() {
            PackageManager::DNF
        } else if Command::new("yum").arg("--version").output().is_ok() {
            PackageManager::YUM
        } else if Command::new("zypper").arg("--version").output().is_ok() {
            PackageManager::Zypper
        } else if Command::new("pacman").arg("--version").output().is_ok() {
            PackageManager::Pacman
        } else {
            PackageManager::Unknown
        }
    }
}

pub fn check_if_guestmount_is_installed() -> bool {
    match Command::new("guestmount").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                // If the command was successful, guestmount is installed
                return true;
            } else {
                // If the command failed, guestmount is not installed
                return false;
            }
        }
        Err(_) => {
            // If there was an error running the command, guestmount is not installed
            return false;
        }
    } 
}

pub fn download_guestmount_if_not_present() -> Result<(), String> {
    if check_if_guestmount_is_installed() {
        return Ok(());
    }

    let package_manager = PackageManager::detect();
    let (cmd, args): (&str, Vec<&str>) = match package_manager {
        PackageManager::APT => ("sudo", vec!["apt-get", "update", "&&",
            "sudo", "apt-get", "install", "-y", "guestmount"]),
        PackageManager::DNF => ("sudo", vec!["dnf", "install", "-y",
            "guestmount"]),
        PackageManager::YUM => ("sudo", vec!["yum", "install", 
            "-y", "guestmount"]),
        PackageManager::Zypper => ("sudo", vec!["zypper", "install",
            "-y", "guestmount"]),
        PackageManager::Pacman => ("sudo", vec!["pacman", "-Sy", 
            "--noconfirm", "guestmount"]),
        PackageManager::Unknown => {
            return Err(
                "Unsupported package manager. Please install 'guestmount' manually.".to_string()
            );
        }
    };
    
    if let PackageManager::APT = package_manager {
        let status = Command::new("sh")
            .arg("-c")
            .arg("sudo apt-get update && sudo apt-get install -y guestmount")
            .status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("Failed to install guestmount (exit code: {})", s)),
            Err(e) => Err(format!("Failed to run apt-get: {}", e)),
        }
    } else {
        let status = Command::new(cmd)
            .args(&args[1..])
            .status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("Failed to install guestmount (exit code: {})", s)),
            Err(e) => Err(format!("Failed to run {}: {}", cmd, e)),
        }
    }
}