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

    pub fn from_boxed_fn(
        code: Box<Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken>>,
        arity: usize,
        closure: Scope,
    ) -> Function {
        Function {
            code: Code {
                function: code.into(),
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

    pub fn call_in_frame(
        &self,
        interpreter: &mut Interpreter,
        args: &[ObjectToken],
    ) -> Result<Option<ObjectToken>, TriconeError> {
        if self.arity == args.len() {
            Ok(interpreter.with_new_scope(|interpreter| (self.code.function)(interpreter, args)))
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

pub fn register_func_type(interpreter: &mut Interpreter, module: &mut Module) {
    generic::create_type_for::<Function, _>(interpreter, module, "Function", |_, _, ty| {
        // The interpreter needs to know if an object is a function object easily
        assert_eq!(ty.index, consts::FUNCTION_TYPE_ID);
    });
}

pub fn function_object_from_function(func: Function) -> ObjectToken {
    ObjectToken::new(unsafe { generic::create_object_from_val(consts::FUNCTION_TYPE_ID, func) })
}

pub fn function_from_function_object(obj: &Object) -> &Function {
    if obj.type_ != consts::FUNCTION_TYPE_ID {
        panic!("Not a function object!");
    }
    // TODO: Needs drop implementation
    unsafe { generic::get_unsafe_ref(obj) }
}
