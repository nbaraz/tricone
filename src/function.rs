use generic;
use interpreter::*;

use std::rc::Rc;

pub struct Code {
    function: Rc<Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken>>,
}

impl Code {
    pub fn create(instructions: Vec<Instruction>) -> Code {
        Code {
            function: Rc::new(move |interpreter, _args| interpreter.run_code(&instructions)),
        }
    }
}

pub struct Function {
    code: Code,
    arity: usize,
    pub closure: Scope,
}

impl Function {
    pub fn new<F>(code: F, arity: usize, closure: Scope) -> Function
    where
        F: Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken> + 'static,
    {
        Function {
            code: Code {
                function: Rc::new(code),
            },
            arity,
            closure,
        }
    }

    pub fn from_code(code: Code, arity: usize, closure: Scope) -> Function {
        Function {
            code,
            arity,
            closure,
        }
    }

    pub fn dup(&self) -> Function {
        Function {
            code: Code {
                function: Rc::clone(&self.code.function),
            },
            arity: self.arity,
            closure: self.closure.dup(),
        }
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        args: &[ObjectToken],
    ) -> Result<Option<ObjectToken>, TriconeError> {
        if self.arity == args.len() {
            Ok(
                interpreter.with_new_frame(self.closure.dup(), |interpreter| {
                    (self.code.function)(interpreter, args)
                }),
            )
        } else {
            Err(TriconeError {
                kind: ErrorKind::WrongArgumentCount,
            })
        }
    }
}

impl generic::TriconeDefault for Function {
    fn tricone_default() -> Function {
        Function::new(
            move |_, _| panic!("Uninitialized function"),
            0,
            Scope::new(),
        )
    }
}

pub fn register_func_type(interpreter: &mut Interpreter) -> TypeIndex {
    let func_ty = generic::create_type_for::<Function>("Function");
    interpreter.register_type(consts::CORE_MODULE_ID, func_ty)
}

pub fn function_from_function_object(obj: &Object) -> &Function {
    if obj.type_ != consts::FUNCTION_TYPE_ID {
        panic!("Not a function object!");
    }
    // TODO: Needs drop implementation
    unsafe { generic::get_unsafe_ref(obj) }
}
