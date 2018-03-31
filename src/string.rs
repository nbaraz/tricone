use generic;
use interpreter::*;

pub fn register_string_type(interpreter: &mut Interpreter) {
    let mut string_ty = generic::create_type_for::<String>("String");

    string_ty.register_method("println", 0, move |_itrp, args| {
        let target = args[0].obj();
        println!("{}", unsafe { generic::get_unsafe_ref::<String>(&target) });
        None
    });

    interpreter.register_type(consts::CORE_MODULE_ID, string_ty);
}

pub fn create_string(interpreter: &mut Interpreter, value: String) -> ObjectToken {
    let tyidx = interpreter
        .lookup_type(consts::CORE_MODULE_ID, "String")
        .unwrap();
    let token = interpreter.create_object(tyidx);
    {
        let mut obj = token.obj_mut();
        unsafe { generic::put_unsafe(&mut obj, value) }
    }
    token
}
