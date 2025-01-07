#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub process_id: u64,

    pub address: *mut u8,
    pub buffer: *mut u8,

    pub size: usize,
}
