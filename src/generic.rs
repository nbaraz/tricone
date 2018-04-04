use interpreter::*;

use std::mem;
use std::ops::Add;
use std::ptr;

pub unsafe fn get_unsafe_copy<T: Copy>(obj: &Object) -> T {
    *get_unsafe_ref(obj)
}

pub unsafe fn get_unsafe_ref<T>(obj: &Object) -> &T {
    assert_eq!(obj.data.len(), aligned_allocation_size::<T>());
    &*(align_pointer::<T>(obj.data.as_ptr() as usize) as *const T)
}

pub unsafe fn get_unsafe_mut<T>(obj: &mut Object) -> &mut T {
    assert_eq!(obj.data.len(), aligned_allocation_size::<T>());
    &mut *(align_pointer::<T>(obj.data.as_mut_ptr() as usize) as *mut T)
}

pub unsafe fn put_unsafe<T>(obj: &mut Object, val: T) {
    *get_unsafe_mut(obj) = val;
}

fn align_pointer<T>(pointer: usize) -> usize {
    let align = mem::align_of::<T>();
    (pointer + align - 1) & !(align - 1)
}

fn aligned_allocation_size<T>() -> usize {
    mem::size_of::<T>() + mem::align_of::<T>() - 1
}

pub fn create_type_for<T: TriconeDefault, F>(
    interpreter: &mut Interpreter,
    module: &mut Module,
    name: &str,
    with_ty: F,
) where
    F: FnOnce(&mut Interpreter, &mut Module, &mut Type),
{
    module.create_type(interpreter, name, |interpreter, module, ty| {
        // TODO: make this a 'static method'
        ty.register_method(consts::CREATE_METHOD_NAME, 0, move |_itrp, args| {
            let mut target = args[0].obj_mut();
            let mut data = Vec::with_capacity(aligned_allocation_size::<T>());

            unsafe {
                data.set_len(aligned_allocation_size::<T>());
                target.data = data;
                put_unsafe(&mut target, <T as TriconeDefault>::tricone_default());
            }

            None
        });

        ty.register_method(consts::DROP_METHOD_NAME, 0, move |_itrp, args| {
            let mut target = args[0].obj_mut();
            unsafe {
                ptr::drop_in_place(get_unsafe_mut::<T>(&mut target) as *mut T);
            }
            None
        });

        (with_ty)(interpreter, module, ty);
    });
}

pub fn impl_add_for<T: Add + Clone>(ty: &mut Type) {
    ty.register_method("add", 1, move |itrp, args| {
        let a = args[0].obj();
        let b = args[1].obj();

        assert_eq!(a.type_, b.type_);

        let res_obj = itrp.create_object(a.type_, 0);
        unsafe {
            let mut res_ = res_obj.obj_mut();
            let (val_a, val_b): (&T, &T) = (get_unsafe_ref(&a), get_unsafe_ref(&b));
            put_unsafe(&mut res_, Add::add(val_a.clone(), val_b.clone()));
        }

        Some(res_obj)
    });
}

pub trait TriconeDefault: Sized {
    fn tricone_default() -> Self;
}

impl<T> TriconeDefault for T
where
    T: Default,
{
    fn tricone_default() -> Self {
        Default::default()
    }
}
