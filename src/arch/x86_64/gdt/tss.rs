/// A Task State Segment
/// This was used in 32-bit x86 for hardware context switching. In 64-bit mode it is only used
/// for switching stacks on priviledge change or interrupt.
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Tss {
    _reserved1: u32,

    /// Stack pointers to different stacks to be used on a priviledge level change.
    pub priviledge_stacks: [u64; 3],

    _reserved2: u64,

    /// Stack pointers to different stacks to be used in an interrupt handler.
    /// Registered to the handler in its IDT descriptor.
    pub interrupt_stacks: [u64; 7],

    _reserved3: u64,
    _reserved4: u16,

    /// I/O permission bitmap pointer. Currently unused.
    pub io_permission_base: u16,
}

impl Tss {
    pub fn new() -> Tss {
        Tss {
            _reserved1: 0,
            priviledge_stacks: [0; 3],
            _reserved2: 0,
            interrupt_stacks: [0; 7],
            _reserved3: 0,
            _reserved4: 0,
            io_permission_base: 0,
        }
    }
}
