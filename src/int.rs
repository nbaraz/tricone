use interpreter::*;
use generic;

pub fn register_int_type(interpreter: &mut Interpreter) {
    let mut int_ty = generic::create_type_for::<i64>(interpreter, "Int");
    generic::impl_add_for::<i64>(&mut int_ty);

    interpreter.register_type(interpreter_consts::CORE_MODULE_ID, int_ty);
}
