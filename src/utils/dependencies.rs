use std::process::Command;

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