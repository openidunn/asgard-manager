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