/// Represents the components required to boot a Linux kernel in a virtual machine.
///
/// This struct encapsulates the raw bytes of the kernel image and optionally the
/// initial RAM disk (initrd). These components are typically loaded into guest
/// memory prior to boot.
///
/// # Fields
/// * `kernel` - The raw kernel binary as a byte vector (usually vmlinux or bzImage).
/// * `initrd` - Optional byte vector containing an initrd image (e.g., initramfs),
///              which provides a temporary root filesystem during early boot.
#[derive(Debug)]
pub struct KernelComponents {
    pub kernel: Vec<u8>,             // Raw contents of the kernel image
    pub initrd: Option<Vec<u8>>      // Optional initrd/initramfs contents
}