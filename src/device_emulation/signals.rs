use vmm_sys_util::eventfd::EventFd;
use kvm_ioctls::VmFd;

pub struct Interrupt {
    irqfd: EventFd,     // eventfd used for signaling interrupt
    vm_fd: VmFd,        // handle to KVM VM for ioctl calls
    gsi: u32,           // guest interrupt number (IRQ line)
}

impl Interrupt {
    pub fn new(vm_fd: VmFd, gsi: u32) -> Result<Self, String> {
        let irqfd = match EventFd::new(0) {
            Ok(e) => e,
            Err(e) => return Err(format!("{:?}", e))
        };
        match vm_fd.register_irqfd(&irqfd, gsi) {
            Ok(_) => Ok(Interrupt { irqfd, vm_fd, gsi }),
            Err(e) => return Err(format!("{:?}", e))
        }
    }

    pub fn trigger(&self) -> Result<(), String> {
        match self.irqfd.write(1) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("{:?}", e))
        }
    }

    pub fn get_irqfd(&self) -> &EventFd {
        &self.irqfd
    }

    pub fn get_vm(&self) -> &VmFd {
        &self.vm_fd
    }

    pub fn get_gsi(&self) -> u32 {
        self.gsi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kvm_ioctls::{Kvm, VmFd};
    use vmm_sys_util::eventfd::EventFd;

    // Helper to create a VmFd with IRQ chip initialized
    fn create_vm_fd() -> VmFd {
        let kvm = Kvm::new().expect("Failed to open /dev/kvm");
        let vm = kvm.create_vm().expect("Failed to create VM");
        // Create IRQ chip (required before registering irqfds)
        #[cfg(target_arch = "x86_64")]
        vm.create_irq_chip().expect("Failed to create IRQ chip");
        vm
    }

    #[test]
    fn test_interrupt_new_success() {
        let vm_fd = create_vm_fd();
        let gsi = 5; // arbitrary GSI number

        let interrupt = Interrupt::new(vm_fd, gsi);
        assert!(interrupt.is_ok(), "Interrupt::new should succeed");
    }

    #[test]
    fn test_interrupt_trigger() {
        let vm_fd = create_vm_fd();
        let gsi = 5;

        let interrupt = Interrupt::new(vm_fd, gsi).expect("Failed to create Interrupt");

        // Trigger the interrupt, should succeed
        let result = interrupt.trigger();
        assert!(result.is_ok(), "Interrupt::trigger should succeed");
    }
}
