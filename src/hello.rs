use function::{Code, Function};
use interpreter::*;
use moduledef::*;

use std::collections::HashMap;
use std::iter::FromIterator;

fn register_hello(interpreter: &mut Interpreter) {
    use interpreter::Instruction::*;
    let def = ModuleDef {
        name: "hello".to_owned(),
        types: HashMap::from_iter(vec![(
            "Hello".to_owned(),
            TypeDef {
                methods: HashMap::from_iter(vec![
                    (
                        "hello".to_owned(),
                        FunctionDef::Native(NativeFunctionDef {
                            arity: 1,
                            code: Box::new(move |_itrp, _args| {
                                println!("hello from method!!");
                                None
                            }),
                        }),
                    ),
                    (
                        consts::CREATE_METHOD_NAME.to_owned(),
                        FunctionDef::Native(NativeFunctionDef {
                            arity: 1,
                            code: Box::new(move |_itrp, _args| {
                                println!("hello from CREATE method!!");
                                None
                            }),
                        }),
                    ),
                    (
                        consts::DROP_METHOD_NAME.to_owned(),
                        FunctionDef::Native(NativeFunctionDef {
                            arity: 1,
                            code: Box::new(move |_itrp, _args| {
                                println!("hello from DROP method!!");
                                None
                            }),
                        }),
                    ),
                ]),
            },
        )]),
        free_functions: HashMap::from_iter(vec![
            (
                "hello".to_owned(),
                FunctionDef::Bytecode(BytecodeFunctionDef {
                    arity: 0,
                    instructions: vec![
                        CreateObject {
                            type_spec: ("hello".to_owned(), "Hello".to_owned()),
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
                        LookupName {
                            name: "do_add".to_owned(),
                        },
                        Assign {
                            name: "do_add_2".to_owned(),
                        },
                        LookupName {
                            name: "do_add_2".to_owned(),
                        },
                        CallFunctionObject {
                            num_args: 0,
                            use_result: false,
                        },
                        GetTopScope,
                        Assign {
                            name: "this".to_owned(),
                        },
                        GetTopScope,
                        DebugPrintObject,
                    ],
                }),
            ),
            (
                "do_add".to_owned(),
                FunctionDef::Bytecode(BytecodeFunctionDef {
                    arity: 0,
                    instructions: vec![
                        CreateInt { value: 20 },
                        CreateInt { value: 22 },
                        CallMethod {
                            name: "add".to_owned(),
                            num_args: 1,
                            use_result: true,
                        },
                        CallMethod {
                            name: "tostring".to_owned(),
                            num_args: 0,
                            use_result: true,
                        },
                        CallMethod {
                            name: "println".to_owned(),
                            num_args: 0,
                            use_result: false,
                        },
                        Diag,
                    ],
                }),
            ),
        ]),
    };

    def.register(interpreter);
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
            GetModuleGlobals {
                name: "hello".to_owned(),
            },
            GetMember {
                name: "do_add".to_owned(),
            },
            CallFunctionObject {
                num_args: 0,
                use_result: false,
            },
        ]),
        0,
        Scope::new(),
    );

    let obj = func.call(interpreter, &[]).unwrap();
    match obj {
        None => {}
        Some(obj) => {
            println!("Should be unreachable");
            interpreter.drop_token(obj);
        }
    }
    interpreter.drop_token(func.closure.vars);
}
