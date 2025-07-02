use virtio_bindings::virtio_mmio::VIRTIO_MMIO_QUEUE_NOTIFY;
use virtio_bindings::virtio_blk::*;
use virtio_queue::{QueueT, QueueSync};
use vm_memory::{Bytes, GuestMemoryMmap, Address};
use memmap2::MmapMut;
use std::cell::RefCell;
use super::super::super::utils::signals::linux::Interrupt;

/// Virtio block device implementation using MMIO transport.
/// Handles guest memory, disk image backing, virtio queue, and interrupts.
pub struct VirtioBlockDevice {
    /// Guest physical memory mapping
    pub mem: RefCell<GuestMemoryMmap>,
    /// Memory-mapped disk image file backing the block device
    pub disk_image: RefCell<MmapMut>,
    /// Base MMIO address of the device
    pub mmio_base: u64,
    /// Virtio queue synchronized structure, representing the virtqueue used for I/O requests
    pub queue: RefCell<QueueSync>, // set up when guest writes to MMIO
    /// Interrupt controller abstraction to raise interrupts on behalf of the device
    pub interrupt_controller: Interrupt
}

impl VirtioBlockDevice {
    /// Creates a new VirtioBlockDevice instance.
    ///
    /// Initializes the virtqueue with preset descriptor, avail ring, and used ring addresses.
    /// Validates the queue using the guest memory mapping.
    ///
    /// # Arguments
    /// * `mem` - Guest physical memory
    /// * `disk_image` - Memory mapped backing storage for the block device
    /// * `mmio_base` - Base address for MMIO registers
    /// * `interrupt_controller` - Interrupt handler abstraction
    ///
    /// # Returns
    /// * `Ok(Self)` on success
    /// * `Err(String)` on failure (e.g., queue initialization failure or invalid queue)
    pub fn new(mem: GuestMemoryMmap, disk_image: MmapMut, mmio_base: u64, interrupt_controller: Interrupt) -> Result<Self, String> {
        // Initialize virtqueue with 1024 descriptors
        let mut queue = match QueueSync::new(1024) {
            Ok(q) => q,
            Err(e) => return Err(format!("{:?}", e))
        };

        // Hardcoded addresses for queue structures in guest memory (example values)
        let desc_table_addr: u64 = 0x1000;
        let avail_ring_addr: u64 = 0x2000;
        let used_ring_addr: u64 = 0x3000;

        // Set descriptor table address (split 64-bit into two 32-bit parts)
        queue.set_desc_table_address(Some((desc_table_addr & 0xFFFFFFFF) as u32), Some((desc_table_addr >> 32) as u32));
        // Set available ring address
        queue.set_avail_ring_address(Some((avail_ring_addr & 0xFFFFFFFF) as u32), Some((avail_ring_addr >> 32) as u32));
        // Set used ring address
        queue.set_used_ring_address(Some((used_ring_addr & 0xFFFFFFFF) as u32), Some((used_ring_addr >> 32) as u32));
        queue.set_ready(true);

        // Verify queue validity against the guest memory layout
        if !queue.is_valid(&mem) {
            return Err(format!("queue is invalid"));
        }

        // Return the new block device instance with initialized fields
        Ok(Self {
            mem: RefCell::new(mem),
            disk_image: RefCell::new(disk_image),
            mmio_base,
            queue: RefCell::new(queue), // max 1024 descriptors
            interrupt_controller,
        })
    }

    /// Reads a 32-bit MMIO register at the given offset.
    ///
    /// Returns device-specific values depending on the offset.
    /// For simplicity, only a few standard registers are implemented.
    ///
    /// # Arguments
    /// * `offset` - Offset of the MMIO register from base
    ///
    /// # Returns
    /// * The 32-bit value read from the device register
    pub fn read_mmio(&self, offset: u64) -> u32 {
        match offset {
            0x000 => 0x74726976,       // Magic value "virt" (0x74726976 in hex)
            0x004 => 2,                // Version (virtio version 2)
            0x008 => 2,                // Device ID: 2 for block device
            0x00c => 0x554d4551,       // Vendor ID "QEMU"
            0x010 => 0,                // Host features (none currently implemented)
            _ => 0,                    // Default for other registers
        }
    }

    /// Writes to a 32-bit MMIO register at the given offset.
    ///
    /// For now, only the queue notify register is handled. Other writes are ignored.
    ///
    /// # Arguments
    /// * `offset` - Offset of the MMIO register from base
    pub fn write_mmio(&self, offset: u64) {
        if offset == (VIRTIO_MMIO_QUEUE_NOTIFY as u64) {
            // Guest notified device that there are new buffers in the virtqueue
            self.process_descriptor_chain();
        }
        else {
            // Other writes ignored for simplicity
        }
    }

    /// Processes descriptor chains from the virtqueue.
    ///
    /// Iterates over available descriptors, interprets block requests (read/write),
    /// performs I/O on the backing disk image, updates used ring, writes status,
    /// and triggers interrupts if needed.
    pub fn process_descriptor_chain(&self) {
        let memory = self.mem.borrow_mut();
        let mut que = self.queue.borrow_mut();

        // If queue not ready, no processing possible
        if !que.ready() {
            return;
        }

        // Process each available descriptor chain
        while let Some(descriptor_chain) = que.pop_descriptor_chain(&*memory) {
            // Head descriptor index, needed for used ring update
            let head_index = descriptor_chain.head_index();
            let mut desc_iter = descriptor_chain.into_iter();

            // The first descriptor contains the request header
            let header_descriptor = match desc_iter.next() {
                Some(h) => h,
                None => return 
            };

            // Read request type from header (e.g., VIRTIO_BLK_T_IN or VIRTIO_BLK_T_OUT)
            let request_type = match memory.read_obj::<u32>(header_descriptor.addr()) {
                Ok(r) => r,
                Err(_) => return
            };

            // The sector number is stored 8 bytes after the start of the header descriptor
            let sector_address = match header_descriptor.addr().checked_add(8) {
                Some(a) => a,
                None => return
            };

            // Read sector number from guest memory
            let sector = match memory.read_obj::<u64>(sector_address) {
                Ok(s) => s,
                Err(_) => return
            };

            // The second descriptor points to the data buffer (either source or destination)
            let data_descriptor = match desc_iter.next() {
                Some(d) => d,
                None => return
            };

            let mut disk_img = self.disk_image.borrow_mut();

            match request_type {
                VIRTIO_BLK_T_IN => {
                    // Handle read request: copy data from disk to guest buffer
                    let sector_offset = sector * 512;
                    let data = &disk_img[(sector_offset as usize)..(sector_offset + data_descriptor.len() as u64) as usize];
                    if let Err(_) = memory.write_slice(data, data_descriptor.addr()) {
                        return;
                    };
                }
                VIRTIO_BLK_T_OUT => {
                    // Handle write request: copy data from guest buffer to disk
                    let sector_offset = sector * 512;
                    let mut buffer = vec![0u8; data_descriptor.len() as usize];
                    if let Err(_) = memory.read_slice(&mut buffer, data_descriptor.addr()) {
                        return;
                    };
                    disk_img[sector_offset as usize..(sector_offset + data_descriptor.len() as u64) as usize]
                        .copy_from_slice(&buffer);
                }
                _ => {}
            }

            // The last descriptor is used to return the status byte to the guest
            let status_descriptor = match desc_iter.next() {
                Some(s) => s,
                None => return
            };

            // Write status = 0 (success) to the status descriptor buffer
            if let Err(_) = memory.write_obj(0u8, status_descriptor.addr()) {
                return;
            };

            // Add the processed descriptor to the used ring with the length of the data buffer
            if let Err(_) = que.add_used(&*memory, head_index, data_descriptor.len()) {
                return;
            }

            // Check if guest requested notification; if yes, trigger interrupt
            match que.needs_notification(&*memory) {
                Ok(b) => {
                    if b {
                        if let Err(_) = self.interrupt_controller.trigger() {
                            return;
                        };
                    }
                }
                Err(_) => return
            }
        }
    }
}