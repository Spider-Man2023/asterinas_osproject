use alloc::vec::Vec;

use crate::{config::PAGE_SIZE, UPSafeCell};

use super::address::PhysAddr;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSafeCell<FreeListAllocator> = unsafe {
        UPSafeCell::new(FreeListAllocator {
            current: 0,
            end: 0,
            free_list: Vec::new(),
        })
    };
}

trait FrameAllocator {
    fn alloc(&mut self) -> Option<usize>;
    fn dealloc(&mut self, value: usize);
}

pub struct FreeListAllocator {
    current: usize,
    end: usize,
    free_list: Vec<usize>,
}

impl FreeListAllocator {
    fn alloc(&mut self) -> Option<usize> {
        let mut ret = 0;
        if let Some(x) = self.free_list.pop() {
            ret = x;
        } else if self.current < self.end {
            ret = self.current;
            self.current += PAGE_SIZE;
        };
        Some(ret)
    }

    fn dealloc(&mut self, value: usize) {
        assert!(!self.free_list.contains(&value));
        self.free_list.push(value);
    }
}

#[derive(Debug, Clone)]
// #[repr(transparent)]
pub struct PhysFrame {
    start_pa: usize,
}

impl PhysFrame {
    pub const fn start_pa(&self) -> PhysAddr {
        PhysAddr(self.start_pa)
    }

    pub const fn end_pa(&self) -> PhysAddr {
        PhysAddr(self.start_pa + PAGE_SIZE)
    }

    pub fn alloc() -> Option<Self> {
        FRAME_ALLOCATOR
            .exclusive_access()
            .alloc()
            .map(|pa| Self { start_pa: pa })
    }

    pub fn dealloc(pa: usize) {
        FRAME_ALLOCATOR.exclusive_access().dealloc(pa)
    }

    pub fn alloc_zero() -> Option<Self> {
        let mut f = Self::alloc()?;
        f.zero();
        Some(f)
    }

    pub fn zero(&mut self) {
        unsafe { core::ptr::write_bytes(self.start_pa().kvaddr().as_ptr(), 0, PAGE_SIZE) }
    }

    pub fn as_slice(&self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.start_pa().kvaddr().as_ptr(), PAGE_SIZE) }
    }
}

impl Drop for PhysFrame {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.exclusive_access().dealloc(self.start_pa);
    }
}

pub(crate) fn init(start: usize, size: usize) {
    FRAME_ALLOCATOR.exclusive_access().current = start;
    FRAME_ALLOCATOR.exclusive_access().end = start + size;
}
