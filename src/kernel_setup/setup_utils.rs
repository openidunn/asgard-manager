#[derive(Debug)]
pub struct KernelComponents {
    pub kernel: Vec<u8>,
    pub initrd: Option<Vec<u8>>
}