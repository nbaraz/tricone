use generic;
use interpreter::*;

use std::rc::Rc;

pub struct Code {
    function: Rc<Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken>>,
    closure: Scope,
}

impl Code {
    pub fn create(instructions: Vec<Instruction>, closure: Scope) -> Code {
        Code {
            function: Rc::new(move |interpreter, _args| interpreter.run_code(&instructions)),
            closure: closure,
        }
    }
}

pub struct Function {
    code: Code,
    arity: usize,
}

impl Function {
    pub fn new<F>(code: F, arity: usize, closure: Scope) -> Function
    where
        F: Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken> + 'static,
    {
        Function {
            code: Code {
                function: Rc::new(code),
                closure,
            },
            arity,
        }
    }

    pub fn dup(&self) -> Function {
        Function {
            code: Code {
                function: Rc::clone(&self.code.function),
                closure: self.code.closure.dup(),
            },
            arity: self.arity,
        }
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        args: &[ObjectToken],
    ) -> Result<Option<ObjectToken>, TriconeError> {

        // interpreter.thread.

        if self.arity == args.len() {
            Ok((self.code.0)(interpreter, args))
        } else {
            Err(TriconeError {
                kind: ErrorKind::WrongArgumentCount,
            })
        }
    }
}

impl generic::TriconeDefault for Function {
    fn tricone_default() -> Function {
        Function::new(move |_, _| panic!("Uninitialized function"), 0)
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
