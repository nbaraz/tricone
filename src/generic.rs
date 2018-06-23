use interpreter::*;
use string;

use std::fmt;
use std::mem;
use std::ops::Add;
use std::ptr;

pub unsafe fn get_unsafe_copy<T: Copy>(obj: &Object) -> T {
    *get_unsafe_ref(obj)
}

pub unsafe fn get_unsafe_ref<'a, T>(obj: &'a Object) -> &'a T {
    assert_eq!(obj.data.len(), aligned_allocation_size::<T>());
    &*(align_pointer::<T>(obj.data.as_ptr() as usize) as *const T)
}

pub unsafe fn get_unsafe_mut<'a, T>(obj: &mut Object) -> &'a mut T {
    assert_eq!(obj.data.len(), aligned_allocation_size::<T>());
    &mut *(align_pointer::<T>(obj.data.as_mut_ptr() as usize) as *mut T)
}

pub unsafe fn put_unsafe<T>(obj: &mut Object, val: T) {
    ptr::write(get_unsafe_mut(obj) as *mut T, val);
}

fn align_pointer<T>(pointer: usize) -> usize {
    let align = mem::align_of::<T>();
    (pointer + align - 1) & !(align - 1)
}

fn aligned_allocation_size<T>() -> usize {
    mem::size_of::<T>() + mem::align_of::<T>() - 1
}

pub unsafe fn create_object_from_val<T>(ty_idx: TypeIndex, val: T) -> Object {
    let mut obj = Object::raw_new(ty_idx);
    initialize_object_from_val(&mut obj, val);
    obj
}

pub unsafe fn initialize_object_from_val<T>(obj: &mut Object, val: T) {
    let mut data = Vec::with_capacity(aligned_allocation_size::<T>());

    data.set_len(aligned_allocation_size::<T>());
    obj.data = data;
    put_unsafe(obj, val);
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
        ty.register_native_method(consts::CREATE_METHOD_NAME, 1, move |_itrp, args| {
            let mut target = args[0].obj_mut();
            unsafe {
                initialize_object_from_val::<T>(
                    &mut target,
                    <T as TriconeDefault>::tricone_default(),
                )
            };

            None
        });

        ty.register_native_method(consts::DROP_METHOD_NAME, 1, move |_itrp, args| {
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
    ty.register_native_method("add", 2, move |itrp, args| {
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

pub fn impl_display_for<T: fmt::Display>(ty: &mut Type) {
    ty.register_native_method("tostring", 1, move |itrp, args| {
        let obj = args[0].obj();
        Some(string::create_string(
            itrp,
            format!("{}", unsafe { get_unsafe_ref::<T>(&obj) }),
        ))
    });
}

macro_rules! define_core_creator {
    ($def_name:ident, $type:ty, $name:expr) => {
        pub fn $def_name(interpreter: &mut Interpreter, value: $type) -> ObjectToken {
            let tyidx = interpreter
                .lookup_type(consts::CORE_MODULE_ID, $name)
                .unwrap();
            let token = interpreter.create_object(tyidx, 0);
            {
                let mut obj = token.obj_mut();
                unsafe { $crate::generic::put_unsafe(&mut obj, value) }
            }
            token
        }
    };
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
