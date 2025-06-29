use virtio_bindings::virtio_mmio::VIRTIO_MMIO_QUEUE_NOTIFY;
use virtio_bindings::virtio_blk::*;
use virtio_queue::{Queue, QueueT, QueueSync};
use vm_memory::{Bytes, GuestMemoryMmap, Address};
use memmap2::MmapMut;
use std::cell::RefCell;
use super::signals::Interrupt;

pub struct VirtioBlockDevice {
    pub mem: RefCell<GuestMemoryMmap>,
    pub disk_image: RefCell<MmapMut>,
    pub mmio_base: u64,
    pub queue: RefCell<QueueSync>, // set up when guest writes to MMIO
    pub interrupt_controller: Interrupt   // Interrupt wrapping IRQ number to raise
}

impl VirtioBlockDevice {
    pub fn new(mem: GuestMemoryMmap, disk_image: MmapMut, mmio_base: u64, interrupt_controller: Interrupt) -> Result<Self, String> {
        let mut queue = match QueueSync::new(1024) {
            Ok(q) => q,
            Err(e) => return Err(format!("{:?}", e))
        };

        let desc_table_addr: u64 = 0x1000;
        let avail_ring_addr: u64 = 0x2000;
        let used_ring_addr: u64 = 0x3000;
        queue.set_desc_table_address(Some((desc_table_addr & 0xFFFFFFFF) as u32), Some((desc_table_addr >> 32) as u32));
        queue.set_avail_ring_address(Some((avail_ring_addr & 0xFFFFFFFF) as u32), Some((avail_ring_addr >> 32) as u32));
        queue.set_used_ring_address(Some((used_ring_addr & 0xFFFFFFFF) as u32), Some((used_ring_addr >> 32) as u32));
        queue.set_ready(true);

        if !queue.is_valid(&mem) {
            return Err(format!("queue is invalid"));
        }

        Ok(Self {
            mem: RefCell::new(mem),
            disk_image: RefCell::new(disk_image),
            mmio_base,
            queue: RefCell::new(queue), // max 1024 descriptors
            interrupt_controller,
        })
    }

    pub fn read_mmio(&self, offset: u64) -> u32 {
        match offset {
            0x000 => 0x74726976,       // Magic value "virt"
            0x004 => 2,                // Version
            0x008 => 2,                // Device ID: 2 for block
            0x00c => 0x554d4551,       // Vendor ID "QEMU"
            0x010 => 0,                // Host features (for now, none)
            _ => 0,
        }
    }

    pub fn write_mmio(&self, offset: u64) {
        if offset == (VIRTIO_MMIO_QUEUE_NOTIFY as u64) {
            // Queue notify
            self.process_descriptor_chain();
        }
        else {
            // For simplicity, ignore other writes for now
        }
    }

    pub fn process_descriptor_chain(&self) {
        let mut memory = self.mem.borrow_mut();
        let mut que = self.queue.borrow_mut();
        if !que.ready() {
            return;
        }
        while let Some(descriptor_chain) = que.pop_descriptor_chain(&*memory) {
            let head_index = descriptor_chain.head_index();
            let mut desc_iter = descriptor_chain.into_iter();
            let header_descriptor = match desc_iter.next() {
                Some(h) => h,
                None => return 
            };
            // parse request
            let request_type = match memory.read_obj::<u32>(header_descriptor.addr()) {
                Ok(r) => r,
                Err(_) => return
            };

            let sector_address = match header_descriptor.addr().checked_add(8) {
                Some(a) => a,
                None => return
            };

            let sector = match memory.read_obj::<u64>(sector_address) {
                Ok(s) => s,
                Err(_) => return
            };

            // Second descriptor: data buffer
            let data_descriptor = match desc_iter.next() {
                Some(d) => d,
                None => return
            };

            let mut disk_img = self.disk_image.borrow_mut();

            match request_type {
                VIRTIO_BLK_T_IN => {
                    // Read from disk into guest
                    let sector_offset = sector * 512;
                    let data = &disk_img[(sector_offset as usize)..(sector_offset + data_descriptor.len() as u64) as usize];
                    if let Err(_) = memory.write_slice(data, data_descriptor.addr()) {
                        return;
                    };
                }
                VIRTIO_BLK_T_OUT => {
                    // Write from guest into disk
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

            // Last descriptor: status byte
            let status_descriptor = match desc_iter.next() {
                Some(s) => s,
                None => return
            };

            if let Err(_) = memory.write_obj(0u8, status_descriptor.addr()) {
                return;
            };

            if let Err(_) = que.add_used(&*memory, head_index, data_descriptor.len()) {
                return;
            }

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