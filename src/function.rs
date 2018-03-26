use ::*;

pub fn register_func_type(interpreter: &mut Interpreter) -> TypeIndex {
    let func_ty = generic::create_type_for::<Function>(interpreter, "Function");
    interpreter.register_type(interpreter_consts::CORE_MODULE_ID, func_ty)
}

pub fn function_from_function_object<'a>(obj: &'a Object) -> &'a Function {
    if obj.type_ != interpreter_consts::FUNCTION_TYPE_ID {
        panic!("Not a function object!");
    }
    // TODO: Needs drop implementation
    unsafe { generic::get_unsafe_ref(obj) }
}
