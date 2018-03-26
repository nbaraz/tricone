use interpreter::TypeIndex;

use std::cell::{RefCell, Ref, RefMut};
use std::rc::Rc;
use std::collections::HashMap;

pub struct SObject(Rc<RefCell<Object>>);

impl SObject {
    pub fn new(obj: Object) -> SObject {
        SObject(Rc::new(RefCell::new(obj)))
    }

    pub fn obj(&self) -> Ref<Object> {
        self.0.borrow()
    }

    pub fn obj_mut(&self) -> RefMut<Object> {
        self.0.borrow_mut()
    }

    pub fn dup(&self) -> SObject {
        SObject(Rc::clone(&self.0))
    }
}

pub struct Object {
    pub members: HashMap<String, SObject>,
    pub type_: TypeIndex,
    pub data: Vec<u8>,
}
