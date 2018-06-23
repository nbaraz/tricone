use std::collections::HashMap;
use std::iter::FromIterator;

use bool_;
use function::function_from_function_object;
use interpreter::{Interpreter, ObjectToken};
use moduledef::{FunctionDef, ModuleDef, NativeFunctionDef};

fn builtin_if(interpreter: &mut Interpreter, args: &[ObjectToken]) -> Option<ObjectToken> {
    let bool_obj = args[0].obj();

    if *bool_::from_object(interpreter, &bool_obj) {
        let then = args[1].obj();
        function_from_function_object(&then)
            .call(interpreter, &[])
            .unwrap()
    } else {
        let else_ = args[2].obj();
        function_from_function_object(&else_)
            .call(interpreter, &[])
            .unwrap()
    }
}

fn builtin_while(interpreter: &mut Interpreter, args: &[ObjectToken]) -> Option<ObjectToken> {
    let cond_obj = args[0].obj();
    let cond = function_from_function_object(&cond_obj);
    let body_obj = args[1].obj();
    let body = function_from_function_object(&body_obj);

    loop {
        let res_obj = cond.call_in_frame(interpreter, &[]).unwrap().unwrap();
        let res = res_obj.obj();

        if !*bool_::from_object(interpreter, &res) {
            break;
        }

        if let Some(res) = body.call_in_frame(interpreter, args).unwrap() {
            interpreter.drop_token(res);
            panic!("While body should return nothing!");
        }
    }
    None
}

pub fn register_builtins(interpreter: &mut Interpreter) {
    let def = ModuleDef {
        name: "builtins".to_owned(),
        types: HashMap::new(),
        free_functions: HashMap::from_iter(vec![
            (
                "if".to_owned(),
                FunctionDef::Native(NativeFunctionDef {
                    arity: 3,
                    code: Box::new(builtin_if),
                }),
            ),
            (
                "while".to_owned(),
                FunctionDef::Native(NativeFunctionDef {
                    arity: 2,
                    code: Box::new(builtin_while),
                }),
            ),
        ]),
    };

    def.register(interpreter);
}
