use std::{
    alloc,
    cell::Cell,
    ops::{Deref, DerefMut},
    ptr::{self},
    rc::Rc,
};

pub struct ArenaAllocator {
    start: *mut u8,
    end: *mut u8,
    offset: *mut u8,

    entries: Vec<ArenaEntry>,
    validator: Rc<Cell<bool>>,
}

impl ArenaAllocator {
    pub fn new(size_in_bytes: usize) -> Self {
        let layout = alloc::Layout::from_size_align(size_in_bytes, 1).unwrap();
        let start = unsafe { alloc::alloc(layout) };
        let end = unsafe { start.add(size_in_bytes) };

        Self {
            start,
            end,
            offset: start,
            entries: Default::default(),
            validator: Rc::new(Cell::new(true)),
        }
    }

    pub fn alloc<T>(&mut self, f: impl FnOnce() -> T) -> ArenaElement<T> {
        unsafe fn drop<T>(ptr: *mut u8) {
            ptr::drop_in_place(ptr.cast::<T>())
        }

        unsafe {
            let layout = alloc::Layout::new::<T>();
            let offset = self.offset.add(self.offset.align_offset(layout.align()));
            let next_offset = offset.add(layout.size());

            assert!(next_offset <= self.end, "arena out of capacity");

            let thing = ArenaElement::<T> {
                ptr: offset.cast::<T>(),
                valid: self.validator.clone(),
            };

            ptr::write(thing.ptr, f());

            self.entries.push(ArenaEntry {
                ptr: offset,
                drop: drop::<T>,
            });

            self.offset = next_offset;

            thing
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.offset as usize - self.start as usize
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.end as usize - self.start as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        self.validator.set(false);
        self.validator = Rc::new(Cell::new(true));
        self.entries.clear();
        self.offset = self.start;
    }
}

impl Drop for ArenaAllocator {
    fn drop(&mut self) {
        self.clear();

        if !self.start.is_null() {
            let layout = alloc::Layout::from_size_align(self.capacity(), 1).unwrap();
            unsafe {
                alloc::dealloc(self.start, layout);
            }
        }
    }
}

struct ArenaEntry {
    ptr: *mut u8,
    drop: unsafe fn(*mut u8),
}

impl Drop for ArenaEntry {
    fn drop(&mut self) {
        unsafe { (self.drop)(self.ptr) }
    }
}

pub struct ArenaElement<T: ?Sized> {
    ptr: *mut T,
    valid: Rc<Cell<bool>>,
}

impl<T: ?Sized> ArenaElement<T> {
    pub fn map<U>(mut self, map: impl FnOnce(&mut T) -> &mut U) -> ArenaElement<U> {
        ArenaElement {
            ptr: map(&mut self),
            valid: self.valid.clone(),
        }
    }
}

impl<T: ?Sized> Deref for ArenaElement<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        assert!(
            self.valid.get(),
            "Arena element not valid. Arena was cleared"
        );
        unsafe { &*self.ptr }
    }
}
impl<T: ?Sized> DerefMut for ArenaElement<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        assert!(
            self.valid.get(),
            "Arena element not valid. Arena was cleared"
        );
        unsafe { &mut *self.ptr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocation() {
        let mut arena = ArenaAllocator::new(1024);
        let num = arena.alloc(|| 42);
        assert_eq!(*num, 42);
    }

    #[test]
    fn test_arena_capacity() {
        let arena = ArenaAllocator::new(1024);
        assert_eq!(arena.capacity(), 1024);
    }

    #[test]
    fn test_arena_alignment() {
        let mut arena = ArenaAllocator::new(256);
        let a = arena.alloc(|| 1u32);
        let b = arena.alloc(|| 2u8);
        let c = arena.alloc(|| 3u32);

        // Ensure values are correctly allocated
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(*c, 3);

        // Check if pointers are properly aligned
        assert_eq!(
            a.ptr as usize % std::mem::align_of::<u32>(),
            0,
            "u32 is not properly aligned"
        );
        assert_eq!(
            b.ptr as usize % std::mem::align_of::<u8>(),
            0,
            "u8 is not properly aligned"
        );
        assert_eq!(
            c.ptr as usize % std::mem::align_of::<u32>(),
            0,
            "u32 is not properly aligned"
        );
    }

    #[test]
    fn test_arena_len() {
        let mut arena = ArenaAllocator::new(1024);
        assert_eq!(arena.len(), 0);
        let _ = arena.alloc(|| 42u32);
        assert_eq!(arena.len(), 4);
    }

    #[test]
    fn test_arena_clear() {
        let mut arena = ArenaAllocator::new(1024);
        let _num = arena.alloc(|| 42);
        arena.clear();
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
    }

    #[test]
    fn test_arena_multiple_allocations() {
        let mut arena = ArenaAllocator::new(1024);
        let num1 = arena.alloc(|| 10);
        let num2 = arena.alloc(|| 20);
        assert_eq!(*num1, 10);
        assert_eq!(*num2, 20);
    }

    #[test]
    #[should_panic(expected = "arena out of capacity")]
    fn test_arena_out_of_capacity() {
        let mut arena = ArenaAllocator::new(8);
        let _ = arena.alloc(|| [0u8; 16]);
    }

    #[test]
    #[should_panic(expected = "Arena element not valid. Arena was cleared")]
    fn test_arena_use_after_free() {
        let mut arena = ArenaAllocator::new(8);
        let thing = arena.alloc(|| 7u32);
        arena.clear();
        let _thing = *thing;
    }

    #[test]
    fn test_arena_element_map() {
        let mut arena = ArenaAllocator::new(1024);
        let num = arena.alloc(|| 42);
        let mut_ref = num.map(|n| {
            *n += 1;
            n
        });
        assert_eq!(*mut_ref, 43);
    }
}
