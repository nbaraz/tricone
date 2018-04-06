use generic;
use interpreter::*;

pub fn register_string_type(interpreter: &mut Interpreter, module: &mut Module) {
    generic::create_type_for::<String, _>(interpreter, module, "String", |_, _, ty| {
        ty.register_native_method("println", 1, move |_itrp, args| {
            let target = args[0].obj();
            println!("{}", unsafe { generic::get_unsafe_ref::<String>(&target) });
            None
        });
    });
}

define_core_creator!{create_string, String, "String"}
