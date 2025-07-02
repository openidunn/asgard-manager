use vmm_sys_util::eventfd::EventFd;
use kvm_ioctls::VmFd;

/// Struct representing a virtual interrupt mechanism using KVM irqfd.
///
/// This is useful for virtual devices to signal interrupts to the guest OS.
pub struct Interrupt {
    irqfd: EventFd,     // eventfd used for signaling interrupt
    vm_fd: VmFd,        // handle to KVM VM for ioctl calls
    gsi: u32,           // guest interrupt number (IRQ line)
}

impl Interrupt {
    /// Creates a new Interrupt instance by registering an irqfd with KVM.
    ///
    /// # Arguments
    /// * `vm_fd` - Reference to the KVM VM file descriptor.
    /// * `gsi` - Global System Interrupt (GSI) line to trigger in the guest.
    ///
    /// # Returns
    /// A Result containing the initialized Interrupt or a String error.
    pub fn new(vm_fd: VmFd, gsi: u32) -> Result<Self, String> {
        // Create a new eventfd which acts as a signaling mechanism
        let irqfd = match EventFd::new(0) {
            Ok(e) => e,
            Err(e) => return Err(format!("{:?}", e))
        };

        // Register the eventfd with KVM to notify the guest via specified GSI
        match vm_fd.register_irqfd(&irqfd, gsi) {
            Ok(_) => Ok(Interrupt { irqfd, vm_fd, gsi }),
            Err(e) => return Err(format!("{:?}", e))
        }
    }

    /// Triggers the interrupt by writing to the eventfd.
    ///
    /// This signals the guest OS on the specified GSI line.
    pub fn trigger(&self) -> Result<(), String> {
        match self.irqfd.write(1) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("{:?}", e))
        }
    }

    /// Returns a reference to the internal EventFd.
    pub fn get_irqfd(&self) -> &EventFd {
        &self.irqfd
    }

    /// Returns a reference to the associated VmFd.
    pub fn get_vm(&self) -> &VmFd {
        &self.vm_fd
    }

    /// Returns the GSI number associated with this interrupt.
    pub fn get_gsi(&self) -> u32 {
        self.gsi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kvm_ioctls::{Kvm};

    /// Helper function to create a KVM VM instance with IRQ chip initialized.
    ///
    /// This is required to test irqfd registration.
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

        // Should successfully create and register the irqfd
        let interrupt = Interrupt::new(vm_fd, gsi);
        assert!(interrupt.is_ok(), "Interrupt::new should succeed");
    }

    #[test]
    fn test_interrupt_trigger() {
        let vm_fd = create_vm_fd();
        let gsi = 5;

        // Create the interrupt and ensure triggering works
        let interrupt = Interrupt::new(vm_fd, gsi).expect("Failed to create Interrupt");

        // Trigger the interrupt, should succeed
        let result = interrupt.trigger();
        assert!(result.is_ok(), "Interrupt::trigger should succeed");
    }
}