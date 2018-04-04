use function::{function_object_from_function, Code, Function};
use interpreter::*;

fn register_hello(interpreter: &mut Interpreter) -> TypeIndex {
    let (_, ty_idx) = interpreter.create_module("hello", |interpreter, module| {
        let ty_idx = module
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
            .0;

        let globals = module.globals.dup();

        use interpreter::Instruction::*;
        let func = Function::from_code(
            Code::create(vec![
                CreateObject {
                    type_: ty_idx,
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
            ]),
            0,
            globals,
        );
        module.globals.assign_member(
            "hello".to_owned(),
            function_object_from_function(func),
            interpreter,
        );

        ty_idx
    });
    ty_idx
}

pub fn do_hello(interpreter: &mut Interpreter) {
    register_hello(interpreter);

    use interpreter::Instruction::*;
    let func = Function::from_code(
        Code::create(vec![
            GetModuleGlobals {
                name: "hello".to_owned(),
            },
            GetMember {
                name: "hello".to_owned(),
            },
            CallFunctionObject {
                num_args: 0,
                use_result: false,
            },
        ]),
        0,
        Scope::new(),
    );

    let obj = func.call(interpreter, &[]);
    assert!(obj.unwrap().is_none());
    interpreter.drop_token(func.closure.vars);
}
