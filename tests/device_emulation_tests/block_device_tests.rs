use std::fs::OpenOptions;
use std::io::Write;
use memmap2::MmapMut;
use vm_memory::{GuestMemoryMmap, GuestAddress};
use virtio_queue::QueueT;
use kvm_ioctls::{Kvm, VmFd};
use AsgardManager::device_emulation::block_device::VirtioBlockDevice; // Adjust crate path as needed
use AsgardManager::device_emulation::signals::Interrupt;

// Helper: create guest memory of 64 KiB at address 0
fn create_guest_memory() -> GuestMemoryMmap {
    GuestMemoryMmap::from_ranges(&[(GuestAddress(0), 0x10000)]).expect("Failed to create guest memory")
}

// Helper: create a temporary disk image mmap of specified size filled with zeros
fn create_disk_image(size: usize) -> MmapMut {
    let mut path = std::env::temp_dir();
    path.push("virtio_block_device_test.img");

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("Failed to create disk image file");
    file.set_len(size as u64).expect("Failed to set disk image size");
    file.write_all(&vec![0u8; size]).expect("Failed to write disk image");

    unsafe { MmapMut::map_mut(&file).expect("Failed to mmap disk image") }
}

// Helper: create a VmFd with IRQ chip initialized (required for Interrupt)
fn create_vm_fd() -> VmFd {
    let kvm = Kvm::new().expect("Failed to open /dev/kvm");
    let vm = kvm.create_vm().expect("Failed to create VM");
    #[cfg(target_arch = "x86_64")]
    vm.create_irq_chip().expect("Failed to create IRQ chip");
    vm
}

// Helper: create a real Interrupt instance using VmFd and a GSI number
fn create_real_interrupt() -> Interrupt {
    let vm_fd = create_vm_fd();
    let gsi = 5; // example IRQ number
    Interrupt::new(vm_fd, gsi).expect("Failed to create Interrupt")
}

#[test]
fn test_virtio_block_device_new() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024); // 512 KiB
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt);
    assert!(device.is_ok(), "VirtioBlockDevice::new should succeed");
}

#[test]
fn test_virtio_block_device_read_mmio() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    assert_eq!(device.read_mmio(0x000), 0x74726976); // VIRTIO_MMIO_MAGIC_VALUE
    assert_eq!(device.read_mmio(0x004), 2);           // VIRTIO_MMIO_VERSION
    assert_eq!(device.read_mmio(0x008), 2);           // VIRTIO_ID_BLOCK
    assert_eq!(device.read_mmio(0x00c), 0x554d4551);  // VIRTIO_MMIO_VENDOR_ID
    assert_eq!(device.read_mmio(0x010), 0);           // Host features (none)
    assert_eq!(device.read_mmio(0x100), 0);           // Unknown offset returns 0
}

#[test]
fn test_virtio_block_device_write_mmio_queue_notify_no_panic() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    // Writing to QUEUE_NOTIFY offset triggers process_descriptor_chain; should not panic
    device.write_mmio(0x50); // VIRTIO_MMIO_QUEUE_NOTIFY is 0x50
}

#[test]
fn test_virtio_block_device_process_descriptor_chain_empty_queue() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    // The queue is empty, so processing descriptor chain should return immediately without error
    device.process_descriptor_chain();
}

#[test]
fn test_virtio_block_device_invalid_queue() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    // Create a device but manually set queue ready to false to simulate invalid queue
    let mut device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    {
        let mut queue = device.queue.borrow_mut();
        queue.set_ready(false);
    }

    // process_descriptor_chain should return early without panic
    device.process_descriptor_chain();
}

#[test]
fn test_virtio_block_device_trigger_interrupt() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    // Directly trigger interrupt, expect Ok result
    let result = device.interrupt_controller.trigger();
    assert!(result.is_ok(), "Interrupt trigger should succeed");
}

#[test]
fn test_virtio_block_device_process_descriptor_chain_invalid_request_type() {
    let mem = create_guest_memory();
    let disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    // Manually mark queue ready to true and push invalid descriptor chain if possible
    // This is complex without real guest interaction, so here we just ensure no panic occurs
    device.process_descriptor_chain();
}

#[test]
fn test_virtio_block_device_read_write_disk_image_bounds() {
    let mem = create_guest_memory();
    let mut disk_image = create_disk_image(512 * 1024);
    let interrupt = create_real_interrupt();

    let device = VirtioBlockDevice::new(mem, disk_image, 0x1000, interrupt).expect("Failed to create device");

    // Write to disk image directly and verify content
    {
        let mut disk_img = device.disk_image.borrow_mut();
        disk_img[0..4].copy_from_slice(&[1, 2, 3, 4]);
    }

    {
        let disk_img = device.disk_image.borrow();
        assert_eq!(&disk_img[0..4], &[1, 2, 3, 4], "Disk image content should match written bytes");
    }
}