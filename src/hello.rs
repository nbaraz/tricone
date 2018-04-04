use function;
use interpreter::*;

fn register_hello(interpreter: &mut Interpreter) -> TypeIndex {
    let (_, ty_idx) = interpreter.create_module(|interpreter, module| {
        module
            .create_type(interpreter, "Hello", |_, _, ty| {
                ty.register_method("hello", 0, move |_itrp, _args| {
                    println!("hello from method!!");
                    None
                });

                ty.register_method(consts::CREATE_METHOD_NAME, 0, move |_itrp, _args| {
                    println!("hello from CREATE method!!");
                    None
                });

                ty.register_method(consts::DROP_METHOD_NAME, 0, move |_itrp, _args| {
                    println!("hello from DROP method!!");
                    None
                });
            })
            .0
    });
    ty_idx
}

pub fn do_hello(interpreter: &mut Interpreter) {
    let hello_idx = register_hello(interpreter);

    use interpreter::Instruction::*;
    let code = function::Code::create(vec![
        CreateObject {
            type_: hello_idx,
            num_args: 0,
        },
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
    let func = function::Function::from_code(code, 0, Scope::new());
    let obj = func.call(interpreter, &[]);
    assert!(obj.unwrap().is_none());
    interpreter.drop_token(func.closure.vars);
}
