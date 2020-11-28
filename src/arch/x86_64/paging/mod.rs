mod addr;
mod mapper;
mod table;

use core::ptr::Unique;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::arch::instructions::tlb;
pub use addr::*;
use mapper::*;
use table::*;

pub const PAGE_SIZE: usize = 4096;

const PAGE_TABLE_RAW: *mut RecursivePageTable = 0xFFFF_FFFF_FFFF_F000 as *mut RecursivePageTable;

lazy_static! {
    // A referene to the current page table, protected by a Mutex
    pub static ref PAGE_TABLE: Mutex<ActivePageTable> = {
        Mutex::new(ActivePageTable {
            page_table: Unique::new(PAGE_TABLE_RAW).unwrap(),
        })
    };
}

pub struct Page {
    num: usize,
}

impl Page {
    pub fn containing(vaddr: VirtualAddress) -> Page {
        Page {
            num: (vaddr.as_usize() & 0o000000_777_777_777_777_0000) >> 12,
        }
    }

    fn addr(&self) -> VirtualAddress {
        VirtualAddress::from(self.num * PAGE_SIZE)
    }

    // basically , page number is just
    // 0o000000_777_777_777_777_0000 bits in the address
    // So we can utilize same idea for each page table index
    pub fn level4_page_number(&self) -> usize {
        ((self.num >> 27) & 0o777) as usize
    }

    pub fn level3_page_number(&self) -> usize {
        ((self.num >> 18) & 0o777) as usize
    }

    pub fn level2_page_number(&self) -> usize {
        ((self.num >> 9) & 0o777) as usize
    }

    pub fn level1_page_number(&self) -> usize {
        ((self.num >> 0) & 0o777) as usize
    }
}

// Provides the high level interface for modifying the page table
pub struct ActivePageTable {
    pub page_table: Unique<RecursivePageTable>,
}

impl ActivePageTable {
    /// Allows modification to the Page Table. In order to ensure that the changes to the TLB
    /// is flushed properly, modification is only allowed through a closure the user passes in.
    pub fn modify<F>(&mut self, f: F)
    where
        F: FnOnce(Mapper),
    {
        // Safety: This is safe, because we already know by the ActivePageTable invariant
        //         that this table is indeed active, and recursivly mapped.
        let mapper = unsafe { Mapper::new(self.page_table.as_mut()) };

        f(mapper);

        // SAFETY: We are in kernel mode, so this is safe.
        unsafe { tlb::flush() };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_page_numbers() {
        let page = Page {
            num: 0o_246_135_654_321,
        };
        assert_eq!(page.level4_page_number(), 0o246);
        assert_eq!(page.level3_page_number(), 0o135);
        assert_eq!(page.level2_page_number(), 0o654);
        assert_eq!(page.level1_page_number(), 0o321);
    }

    #[test]
    fn page_from_addr() {
        assert_eq!(
            Page::containing(VirtualAddress::from(0x02000 as usize)).num,
            2
        );
        assert_eq!(
            Page::containing(VirtualAddress::from(0x02FFF as usize)).num,
            2
        );
    }
}
