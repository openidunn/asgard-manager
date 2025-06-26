use std::fs::{File, read_dir};
use reqwest::blocking::Client;
use std::env;

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub enum Distribution {
    Debian, Ubuntu, Mint
}

impl Distribution {
    pub fn as_str(&self) -> &str {
        match self {
            Distribution::Debian => "debian",
            Distribution::Ubuntu => "ubuntu",
            Distribution::Mint => "mint"
        }
    }
}

enum Architecture {
    X86,      // 32-bit
    X86_64,   // 64-bit Intel/AMD
    ARM,      // 32-bit ARM
    ARM64,    // aarch64 (Apple Silicon, modern ARM servers)
    Unknown,
}

fn detect_architecture() -> Architecture {
    match env::consts::ARCH {
        "x86" => Architecture::X86,
        "x86_64" => Architecture::X86_64,
        "arm" => Architecture::ARM,
        "aarch64" => Architecture::ARM64,
        _ => Architecture::Unknown
    }
}

fn get_url_to_linux_distribution_download(distribution: Distribution) -> Result<String, String> {
    let cpu_architecture = detect_architecture();

    match cpu_architecture {
        Architecture::X86_64 => match distribution {
            Distribution::Debian => Ok("https://cloud.debian.org/images/cloud/bullseye/latest/debian-11-generic-amd64.qcow2".to_string()),
            Distribution::Ubuntu => Ok("https://cloud-images.ubuntu.com/releases/22.04/release/ubuntu-22.04-server-cloudimg-amd64.img".to_string()),
            Distribution::Mint => Ok("https://mirrors.edge.kernel.org/linuxmint/stable/21.3/linuxmint-21.3-cinnamon-64bit.iso".to_string()),
        },
        Architecture::ARM64 => match distribution {
            Distribution::Debian => Ok("https://cloud.debian.org/images/cloud/bullseye/latest/debian-11-generic-arm64.qcow2".to_string()),
            Distribution::Ubuntu => Ok("https://cloud-images.ubuntu.com/releases/22.04/release/ubuntu-22.04-server-cloudimg-arm64.img".to_string()),
            Distribution::Mint => Err("Linux Mint is not officially available for ARM64 architecture".to_string()),
        },
        _ => Err("Device architecture is not supported for cloud image installation.".to_string()),
    }
}

pub fn check_if_linux_distribution_img_present_in_current_dir(distribution: Distribution) -> Result<(), String> {

    let entries = match read_dir(".") {
        Ok(entries) => entries,
        Err(e) => return Err(format!("{:?}", e))
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => return Err(format!("{:?}", e))
        };
        let filename = entry.file_name().to_string_lossy().into_owned();
        if filename.contains(distribution.as_str()) && filename.ends_with(".img") {
            return Ok(());
        }
    }

    Err(format!("{} image file not found in this directory", distribution.as_str()))
}

pub fn download_linux_lts_image(distribution: Distribution) -> Result<(), String> {
    match check_if_linux_distribution_img_present_in_current_dir(distribution) {
        Ok(_) => {
            let filename = format!("{}-lts.img", distribution.as_str());
            // URL for Ubuntu 22.04 LTS cloud image (amd64)
            let url = match get_url_to_linux_distribution_download(distribution) {
                Ok(url) => url,
                Err(e) => return Err(format!("{:?}", e))
            };
            // Create HTTP client
            let client = Client::new();
            // Send GET request
            let mut response = match client.get(&url).send() {
                Ok(response) => response,
                Err(e) => return Err(format!("{:?}", e))
            };
            // Open file for writing
            let mut file = match File::create(filename) {
                Ok(file) => file,
                Err(e) => return Err(format!("{:?}", e))
            };
            // Copy response bytes to the file
            if let Err(e) = std::io::copy(&mut response, &mut file) {
                return Err(format!("{:?}", e));
            };

            Ok(())
        },
        Err(e) => Err(format!("{:?}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;

    fn setup_temp_test_dir(name: &str) -> (PathBuf, PathBuf) {
        let current_dir = env::current_dir().unwrap();
        let temp_dir = current_dir.join(name);

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }

        fs::create_dir(&temp_dir).unwrap();

        (current_dir, temp_dir)
    }

    fn cleanup_and_restore(original_dir: PathBuf, temp_dir: PathBuf) {
        env::set_current_dir(original_dir).unwrap();
        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_detect_architecture_returns_known_enum() {
        let arch = detect_architecture();
        match arch {
            Architecture::X86 | Architecture::X86_64 | Architecture::ARM | Architecture::ARM64 | Architecture::Unknown => {}
        }
    }

    #[test]
    fn test_get_url_to_linux_distribution_download_known_arch() {
        let result = get_url_to_linux_distribution_download(Distribution::Ubuntu);
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.contains("ubuntu"));
        assert!(url.ends_with(".img") || url.ends_with(".iso") || url.ends_with(".qcow2"));
    }

    #[test]
    fn test_check_if_linux_distribution_img_present_in_current_dir_found() {
        let (original_dir, temp_dir) = setup_temp_test_dir("test_img_present");

        // Create dummy image file
        let file_path = temp_dir.join("ubuntu-lts.img");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "dummy image").unwrap();

        std::env::set_current_dir(&temp_dir).unwrap();
        let result = check_if_linux_distribution_img_present_in_current_dir(Distribution::Ubuntu);
        assert!(result.is_ok());

        cleanup_and_restore(original_dir, temp_dir);
    }

    #[test]
    fn test_check_if_linux_distribution_img_present_in_current_dir_not_found() {
        let (original_dir, temp_dir) = setup_temp_test_dir("test_img_not_present");

        std::env::set_current_dir(&temp_dir).unwrap();
        let result = check_if_linux_distribution_img_present_in_current_dir(Distribution::Mint);
        assert!(result.is_err());

        cleanup_and_restore(original_dir, temp_dir);
    }

    #[test]
    fn test_distribution_as_str() {
        assert_eq!(Distribution::Ubuntu.as_str(), "ubuntu");
        assert_eq!(Distribution::Debian.as_str(), "debian");
        assert_eq!(Distribution::Mint.as_str(), "mint");
    }

    #[test]
    fn test_check_if_linux_distribution_img_present_with_similar_names_but_wrong_extension() {
        let (original_dir, temp_dir) = setup_temp_test_dir("test_img_similar_name_wrong_extension");

        // Create a file that contains the distro name but wrong extension
        let file_path = temp_dir.join("ubuntu-lts.iso");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "dummy iso").unwrap();

        env::set_current_dir(&temp_dir).unwrap();

        // Should not find .img files, so returns Err
        let result = check_if_linux_distribution_img_present_in_current_dir(Distribution::Ubuntu);
        assert!(result.is_err());

        cleanup_and_restore(original_dir, temp_dir);
    }

    #[test]
    fn test_check_if_linux_distribution_img_present_case_sensitive() {
        let (original_dir, temp_dir) = setup_temp_test_dir("test_img_case_sensitive");

        // Create .img file with uppercase distribution name (should NOT match if case-sensitive)
        let file_path = temp_dir.join("Ubuntu-lts.img");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "dummy img").unwrap();

        env::set_current_dir(&temp_dir).unwrap();

        // Should NOT find, assuming case-sensitive matching
        let result = check_if_linux_distribution_img_present_in_current_dir(Distribution::Ubuntu);
        assert!(result.is_err());

        cleanup_and_restore(original_dir, temp_dir);
    }
}