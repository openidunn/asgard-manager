use std::fs::{File, OpenOptions};
use memmap2::{MmapOptions, MmapMut};

/// Creates a disk image file with the specified path and size.
/// 
/// The file will be created with a `.img` extension and truncated or extended
/// to the requested size. Useful for creating backing storage for virtual block devices.
///
/// # Arguments
/// * `path` - Base file path without extension
/// * `size` - Desired size of the disk image in bytes
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` if the file couldn't be created or resized
fn create_disk_image(path: &str, size: u64) -> Result<(), String> {
    let path_with_img_extension = format!("{}{}", path, ".img"); // Append `.img` to the filename

    // Try opening the file for writing, creating it if it doesn't exist
    let file = match OpenOptions::new()
        .write(true)          // Open the file with write access
        .create(true)         // Create the file if it doesn't exist
        .open(&path_with_img_extension.as_str()) { // Try to open it, or return an error
        Ok(file) => file,
        Err(e) => return Err(format!("{:?}", e))
    };         

    // Resize the file to the requested length in bytes
    if let Err(e) = file.set_len(size) {
        return Err(format!("{:?}", e));
    }      

    Ok(())
}

/// Memory-maps a disk image as a mutable buffer for direct access.
///
/// Ensures the file ends in `.img` before attempting to open and map it.
///
/// # Arguments
/// * `path` - Path to a `.img` disk image file
///
/// # Returns
/// * `Ok(MmapMut)` containing the memory-mapped contents of the image
/// * `Err(String)` if file access or mapping fails
pub fn map_disk_image(path: &str) -> Result<MmapMut, String> {
    // Validate file extension
    if !path.ends_with(".img") {
        return Err(format!("passed path is not path to .img file"));
    }

    // Open file for both reading and writing
    let file = match File::options().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(e) => return Err(format!("{:?}", e))
    };

    // Map the file into memory as a writable buffer
    match unsafe { MmapOptions::new().map_mut(&file) } {
        Ok(mmap) => Ok(mmap),
        Err(e) => Err(format!("{:?}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, metadata};
    use std::io::{Write, Seek, SeekFrom};

    const TEST_FILE: &str = "test_disk.img"; // Used by multiple tests

    #[test]
    fn test_create_disk_image_success() {
        let path = "test_file";
        let full_path = format!("{}.img", path);
        let size = 1024 * 1024; // 1MB

        // Clean up before test
        let _ = remove_file(&full_path);

        // Create image and verify success
        let result = create_disk_image(path, size);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        // Validate file exists and size matches
        let meta = metadata(&full_path).expect("File should exist");
        assert_eq!(meta.len(), size);

        // Clean up after test
        let _ = remove_file(&full_path);
    }

    #[test]
    fn test_create_disk_image_invalid_path() {
        // Try to create file in a non-existent directory
        let path = "/invalid/path/to/file";
        let result = create_disk_image(path, 1024);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_disk_image_overwrites_existing() {
        let path = "overwrite_test";
        let full_path = format!("{}.img", path);
        let _ = remove_file(&full_path);

        // First create small file
        create_disk_image(path, 512).unwrap();

        // Now overwrite with larger size
        let result = create_disk_image(path, 4096);
        assert!(result.is_ok());

        // Validate the file has the updated size
        let meta = metadata(&full_path).expect("File should exist");
        assert_eq!(meta.len(), 4096);

        // Clean up after test
        let _ = remove_file(&full_path);
    }

    #[test]
    fn test_map_disk_image_success() {
        // Create test disk image file 4KiB
        let mut f = File::create(TEST_FILE).expect("Creating file should succeed");
        f.set_len(4096).expect("Setting length should succeed"); // Set file size

        // Optionally write some initial data to the file
        f.seek(SeekFrom::Start(0)).expect("Moving cursor should succeed");
        f.write_all(&vec![0x00; 4096]).expect("Writing to file should succeed");

        // Try mapping it
        let mut mmap = map_disk_image(TEST_FILE).expect("Mapping should succeed");

        // mmap should have correct length
        assert_eq!(mmap.len(), 4096);

        // Check that contents are what we wrote
        assert_eq!(mmap[0], 0x00);
        assert_eq!(mmap[4095], 0x00);

        // Modify mmap to check write access
        mmap[0] = 100;
        assert_eq!(mmap[0], 100);

        // Cleanup
        remove_file(TEST_FILE).unwrap();
    }

    #[test]
    fn test_map_disk_image_failure_cause_not_existing_file() {
        // Try mapping a file that doesn't exist
        let result = map_disk_image("non_existent_file.img");
        assert!(result.is_err());

        // Error should not be due to extension but due to missing file
        assert!(!result.unwrap_err().contains("passed path is not path to .img file"))
    }

    #[test]
    fn test_map_disk_image_failure_cause_wrong_path() {
        // Path does not end with .img, should return extension error
        let result = map_disk_image("existing_file.png");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("passed path is not path to .img file"));
    }
}