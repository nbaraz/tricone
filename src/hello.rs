use interpreter::*;
use function;

fn register_hello(interpreter: &mut Interpreter) -> TypeIndex {
    let mut hello_ty = Type::new("Hello");

    hello_ty.register_method("hello", 0, move |_itrp, _args| {
        println!("hello from method!!");
        None
    });

    hello_ty.register_method(consts::CREATE_METHOD_NAME, 0, move |_itrp, _args| {
        println!("hello from CREATE method!!");
        None
    });

    hello_ty.register_method(consts::DROP_METHOD_NAME, 0, move |_itrp, _args| {
        println!("hello from DROP method!!");
        None
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
        CreateObject { type_: hello_idx , num_args: 0},
        CallMethod {
            name: "hello".to_owned(),
            num_args: 0,
            use_result: false,
        },
        CreateString {
            value: "Hello world!".to_owned(),
        },
        CallMethod {
            name: "println".to_owned(),
            num_args: 0,
            use_result: false,
        },
        Diag,
    ]);
    let obj = (code)(interpreter, &[]);
    assert!(obj.is_none());
}
