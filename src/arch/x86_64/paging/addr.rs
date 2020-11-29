use super::PAGE_SIZE;

// Represents a 64-bit canonical address
// TODO: it could be cool to associate a vaddr with a page table or something,
//          Could we check if a vaddr is `valid` from original context using this idea?
#[derive(Debug, Copy, Clone)]
pub struct VirtualAddress(u64);

// TODO: refactor conversion methods as traits using macro
#[allow(dead_code)]
impl VirtualAddress {
    pub fn new(raw: u64) -> VirtualAddress {
        // ensure canonical by making sure most sig 16 bits are all 0 or 1
        debug_assert!(
            raw.leading_zeros() > 16 || raw.leading_ones() > 16,
            "Attempt to create non-canonical virtual address!"
        );
        VirtualAddress(raw)
    }

    pub fn new_truncate(addr: u64) -> VirtualAddress {
        VirtualAddress::new((((addr << 16) as isize) >> 16) as u64)
    }

    pub fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub fn as_ptr_mut<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for VirtualAddress {
    fn from(addr: u64) -> Self {
        VirtualAddress::new(addr)
    }
}

impl From<usize> for VirtualAddress {
    fn from(addr: usize) -> Self {
        VirtualAddress::new(addr as u64)
    }
}

impl<T> From<*const T> for VirtualAddress {
    fn from(addr: *const T) -> Self {
        VirtualAddress::new(addr as u64)
    }
}

impl<T> From<&T> for VirtualAddress {
    fn from(addr: &T) -> Self {
        VirtualAddress::from(addr as *const T)
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const fn new(addr: u64) -> PhysicalAddress {
        PhysicalAddress(addr)
    }

    pub fn frame_num(&self) -> usize {
        self.as_usize() / PAGE_SIZE
    }

    pub fn from_frame_num(num: usize) -> PhysicalAddress {
        PhysicalAddress::from(num * PAGE_SIZE)
    }

    pub fn from_usize(addr: usize) -> PhysicalAddress {
        PhysicalAddress::new(addr as u64)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }

    pub fn add(&self, count: u64) -> PhysicalAddress {
        PhysicalAddress::new(self.0 + count)
    }

    pub fn align_up(&self, align: u64) -> PhysicalAddress {
        if align.count_ones() != 1 {
            panic!("Alignment not a power of two!")
        }

        // Have to do this wacky way, otherwise risk of underflowing
        // the address
        let raw = (self.0 + align - 1) & !(align - 1);
        PhysicalAddress::new(raw)
    }
}
impl From<u64> for PhysicalAddress {
    fn from(addr: u64) -> Self {
        PhysicalAddress::new(addr)
    }
}

impl From<usize> for PhysicalAddress {
    fn from(addr: usize) -> Self {
        PhysicalAddress::new(addr as u64)
    }
}

impl<T> From<*const T> for PhysicalAddress {
    fn from(addr: *const T) -> Self {
        PhysicalAddress::new(addr as u64)
    }
}

impl<T> From<&T> for PhysicalAddress {
    fn from(addr: &T) -> Self {
        PhysicalAddress::from(addr as *const T)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[should_panic(expected = "Attempt to create non-canonical virtual address!")]
    fn invalid_addr() {
        VirtualAddress::new(0xFFFF_7AAA_AAAA_AAAA);
    }

    #[test]
    fn valid_addr() {
        VirtualAddress::new(0xFFFF_EAAA_AAAA_AAAA);
    }

    #[test]
    fn truncate() {
        VirtualAddress::new(VirtualAddress::new_truncate(0xFFFF_7AAA_AAAA_AAAA).0);
    }

    #[test]
    fn align() {
        let addr = PhysicalAddress::new(0x3283_2929_1234_1323);
        assert_eq!(
            addr.align_up(4096),
            PhysicalAddress::new(0x3283_2929_1234_2000)
        );
    }
}
