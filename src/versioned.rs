use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

pub struct VersionedValue<T> {
    pub value: T,
    pub version: usize,
}
pub struct Versioned<T> {
    wrapped: Rc<RefCell<VersionedValue<T>>>,
}
impl<T> Versioned<T> {
    pub fn new(value: T) -> Self {
        Self {
            wrapped: Rc::new(RefCell::new(VersionedValue { version: 0, value })),
        }
    }
    pub fn clone(&self) -> Self {
        Versioned {
            wrapped: Rc::clone(&self.wrapped),
        }
    }
    pub fn get_mut(&mut self) -> RefMut<T> {
        let mut versioned = self.wrapped.borrow_mut();
        versioned.version += 1;
        RefMut::map(versioned, |v| &mut v.value)
    }
    pub fn get(&self) -> Ref<VersionedValue<T>> {
        self.wrapped.borrow()
    }
}
