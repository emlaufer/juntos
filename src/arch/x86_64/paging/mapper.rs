use super::table::*;
use super::Page;
use crate::memory::{Frame, FrameAllocator};

pub struct Mapper<'a> {
    page_table: &'a mut RecursivePageTable,
}

impl<'a> Mapper<'a> {
    pub unsafe fn new(table: &mut RecursivePageTable) -> Mapper {
        Mapper { page_table: table }
    }

    pub fn map<A>(&mut self, page: Page, frame: Frame, alloc: &mut A)
    where
        A: FrameAllocator,
    {
        let l4_idx = page.level4_page_number();
        let l3_idx = page.level3_page_number();
        let l2_idx = page.level2_page_number();
        let l1_idx = page.level1_page_number();

        unsafe {
            // have to walk down manually, as just going by vaddr could cause
            // a page fault if not mapped, which we don't want here.
            let l3_table = self.page_table.create_table(l4_idx, alloc);
            let l2_table = l3_table.create_table(l3_idx, alloc);
            let l1_table = l2_table.create_table(l2_idx, alloc);
            // TODO: should actually set flags and stuff based on the frame itself.
            //       for now, this is fine
            l1_table[l1_idx] = Entry::new(&frame, Flags::PRESENT | Flags::WRITE);
        }
    }
    // maps a page to a given frame
    // does not allocate new page tables, i.e. it will
    // return an error if the entire path down the tree isn't allocated
    pub fn map_no_alloc(&mut self, page: Page, frame: Frame) -> Result<(), &str> {
        let l4_idx = page.level4_page_number();
        let l3_idx = page.level3_page_number();
        let l2_idx = page.level2_page_number();
        let l1_idx = page.level1_page_number();

        unsafe {
            // have to walk down manually, as just going by vaddr could cause
            // a page fault if not mapped, which we don't want here.
            let l3_table = self
                .page_table
                .get_table_mut(l4_idx)
                .ok_or("L3 table not mapped")?;
            let l2_table = l3_table
                .get_table_mut(l3_idx)
                .ok_or("L2 table not mapped")?;
            let l1_table = l2_table
                .get_table_mut(l2_idx)
                .ok_or("L1 table not mapped")?;

            // TODO: should actually set flags and stuff based on the frame itself.
            //       for now, this is fine
            l1_table[l1_idx] = Entry::new(&frame, Flags::PRESENT | Flags::WRITE);
        }
        Ok(())
    }

    pub fn unmap(&mut self, page: Page) -> Result<(), &str> {
        let l4_idx = page.level4_page_number();
        let l3_idx = page.level3_page_number();
        let l2_idx = page.level2_page_number();
        let l1_idx = page.level1_page_number();

        unsafe {
            // TODO: Right now, we have no way of actually freeing memory
            //       used by the tables.

            // have to walk down manually, as just going by vaddr could cause
            // a page fault if not mapped, which we don't want here.
            let l3_table = self
                .page_table
                .get_table_mut(l4_idx)
                .ok_or("L3 table not mapped")?;
            let l2_table = l3_table
                .get_table_mut(l3_idx)
                .ok_or("L2 table not mapped")?;
            let l1_table = l2_table
                .get_table_mut(l2_idx)
                .ok_or("L1 table not mapped")?;

            // TODO: should actually set flags and stuff based on the frame itself.
            //       for now, this is fine
            l1_table[l1_idx] = Entry::empty();
        }

        Ok(())
    }
}
