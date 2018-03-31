use interpreter::*;
use function;

fn register_hello(interpreter: &mut Interpreter) -> TypeIndex {
    let mut hello_ty = Type::new("Hello");

    hello_ty.register_method("hello", 0, move |itrp, _args| {
        println!("hello from method!!");
        itrp.get_unit_object()
    });

    hello_ty.register_method(consts::CREATE_METHOD_NAME, 0, move |itrp, _args| {
        println!("hello from CREATE method!!");
        itrp.get_unit_object()
    });

    hello_ty.register_method(consts::DROP_METHOD_NAME, 0, move |itrp, _args| {
        println!("hello from DROP method!!");
        itrp.get_unit_object()
    });

    interpreter.register_type(consts::CORE_MODULE_ID, hello_ty)
}

pub fn do_hello(interpreter: &mut Interpreter) {
    let hello_idx = register_hello(interpreter);
    assert_eq!(
        Some(hello_idx),
        interpreter.lookup_type(consts::CORE_MODULE_ID, "Hello"),
    );

    use interpreter::Instruction::*;
    let code = function::Code::create(vec![
        CreateObject { type_: hello_idx },
        CallMethod {
            name: "hello".to_owned(),
            num_args: 0,
        },
        Pop,
        CreateString {
            value: "Hello world!".to_owned(),
        },
        CallMethod {
            name: "println".to_owned(),
            num_args: 0,
        },
        Pop,
        Diag,
    ]);
    let obj = (code)(interpreter, &[]);
    interpreter.drop_token(obj);
}
