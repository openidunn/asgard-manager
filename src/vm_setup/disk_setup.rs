use std::fs::{File, OpenOptions};
use memmap2::{MmapOptions, MmapMut};

fn create_disk_image(path: &str, size: u64) -> Result<(), String> {
    let path_with_img_extension = format!("{}{}", path, ".img");
    let file = match OpenOptions::new()
        .write(true)          // Open the file with write access
        .create(true)         // Create the file if it doesn't exist
        .open(&path_with_img_extension.as_str()) { // Try to open it, or return an error
        Ok(file) => file,
        Err(e) => return Err(format!("{:?}", e))
    };         

    if let Err(e) = file.set_len(size) { // Set the file's length (size in bytes)
        return Err(format!("{:?}", e));
    }      

    Ok(())
}

pub fn map_disk_image(path: &str) -> Result<MmapMut, String> {
    // Open the disk image file for read and write
    if !path.ends_with(".img") {
        return Err(format!("passed path is not path to .img file"));
    }
    let file = match File::options().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(e) => return Err(format!("{:?}", e))
    };

    // Create a new memory map builder
    // map as mutable so you can write to the disk image
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

    const TEST_FILE: &str = "test_disk.img";

    #[test]
    fn test_create_disk_image_success() {
        let path = "test_file";
        let full_path = format!("{}.img", path);
        let size = 1024 * 1024; // 1MB

        // Clean up before test
        let _ = remove_file(&full_path);

        let result = create_disk_image(path, size);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result);

        let meta = metadata(&full_path).expect("File should exist");
        assert_eq!(meta.len(), size);

        // Clean up after test
        let _ = remove_file(&full_path);
    }

    #[test]
    fn test_create_disk_image_invalid_path() {
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

        let meta = metadata(&full_path).expect("File should exist");
        assert_eq!(meta.len(), 4096);

        let _ = remove_file(&full_path);
    }

    #[test]
    fn test_map_disk_image_success() {
        // Create test disk image file 4KiB
        let mut f = File::create(TEST_FILE).expect("Creating file should succeed");
        f.set_len(4096).expect("Setting length should succeed"); // Set file size
        // Optionally write some initial data
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
        assert!(!result.unwrap_err().contains("passed path is not path to .img file"))
    }

    #[test]
    fn test_map_disk_image_failure_cause_wrong_path() {

        let result = map_disk_image("existing_file.png");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("passed path is not path to .img file"));
    }
}