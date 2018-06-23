use generic;
use interpreter::{consts, Interpreter, Module, ObjectToken};

pub fn register_bool_type(interpreter: &mut Interpreter, module: &mut Module) {
    generic::create_type_for::<bool, _>(interpreter, module, "Bool", |_, _, ty| {
        generic::impl_display_for::<bool>(ty);
    });
}

define_core_creator!{create_bool, bool, "Bool"}
define_into_native!{from_object, bool, "Bool"}
