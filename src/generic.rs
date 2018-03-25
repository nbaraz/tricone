use ::*;

use std::mem;
use std::ops::Add;
use std::ptr;

// TODO: memory alignment

unsafe fn get_unsafe_copy<T: Copy>(obj: &Object) -> T {
    assert_eq!(obj.data.len(), mem::size_of::<T>());
    let pointer: *const T = mem::transmute(obj.data.as_ptr());
    *pointer
}

unsafe fn get_unsafe_ref<'a, T>(obj: &'a Object) -> &'a T {
    assert_eq!(obj.data.len(), mem::size_of::<T>());
    let val: &'a T = mem::transmute(obj.data.as_ptr());
    val
}

unsafe fn get_unsafe_mut<'a, T>(obj: &'a mut Object) -> &'a mut T {
    assert_eq!(obj.data.len(), mem::size_of::<T>());
    let val: &'a mut T = mem::transmute(obj.data.as_mut_ptr());
    val
}

unsafe fn put_unsafe<T>(obj: &mut Object, val: T) {
    assert_eq!(obj.data.len(), mem::size_of::<T>());
    let pointer: *mut T = mem::transmute(obj.data.as_mut_ptr());
    ptr::write(pointer, val);
}

pub fn create_type_for<T>(interpreter: &mut Interpreter, name: &str) -> Type {
    let mut ty = Type {
        name: name.to_owned(),
        methods: HashMap::new(),
    };

    ty.register_method(
        interpreter_consts::INIT_METHOD_NAME,
        0,
        move |itrp, args| {
            let mut target = args[0].obj_mut();
            target.data.resize(mem::size_of::<T>(), 0);
            itrp.get_unit_object()
        },
    );

    ty
}

pub fn impl_add_for<T: Add + Clone>(ty: &mut Type) {
    ty.register_method("add", 1, move |itrp, args| {
        let a = args[0].obj();
        let b = args[1].obj();

        assert_eq!(a.type_, b.type_);

        let res_obj = itrp.create_object(a.type_);
        unsafe {
            let mut res_ = res_obj.obj_mut();
            let (val_a, val_b): (&T, &T) = (get_unsafe_ref(&a), get_unsafe_ref(&b));
            put_unsafe(&mut res_, Add::add(val_a.clone(), val_b.clone()));
        }

        res_obj
    });
}
