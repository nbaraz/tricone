use generic;
use interpreter::{Interpreter, Module};

pub fn register_int_type(interpreter: &mut Interpreter, module: &mut Module) {
    generic::create_type_for::<i64, _>(interpreter, module, "Int", |_, _, ty| {
        generic::impl_add_for::<i64>(ty)
    });
}
