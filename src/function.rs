use interpreter::*;
use generic;

use std::rc::Rc;
use std::ops::Deref;

pub struct Code(Rc<Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken>>);

impl Code {
    pub fn create(instructions: Vec<Instruction>) -> Code {
        Code(Rc::new(move |interpreter, _args| {
            interpreter.run_code(&instructions)
        }))
    }
}

impl Deref for Code {
    type Target = Rc<Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Function {
    code: Code,
    arity: usize,
}

impl Function {
    pub fn new<F>(code: F, arity: usize) -> Function
    where
        F: Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken> + 'static,
    {
        Function {
            code: Code(Rc::new(code)),
            arity,
        }
    }

    pub fn dup(&self) -> Function {
        Function {
            code: Code(Rc::clone(&self.code.0)),
            arity: self.arity,
        }
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        args: &[ObjectToken],
    ) -> Result<Option<ObjectToken>, TriconeError> {
        if self.arity == args.len() {
            Ok((self.code.0)(interpreter, args))
        } else {
            Err(TriconeError {
                kind: ErrorKind::WrongArgumentCount,
            })
        }
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
