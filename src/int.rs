use generic;
use interpreter::{consts, Interpreter, Module, ObjectToken};

pub fn register_int_type(interpreter: &mut Interpreter, module: &mut Module) {
    generic::create_type_for::<i64, _>(interpreter, module, "Int", |_, _, ty| {
        generic::impl_add_for::<i64>(ty);
        generic::impl_display_for::<i64>(ty);
    });
}

define_core_creator!{create_int, i64, "Int"}
