use function::*;
use interpreter::*;

use std::collections::HashMap;

pub struct TypeDef {
    pub methods: HashMap<String, FunctionDef>,
}

pub struct BytecodeFunctionDef {
    pub arity: usize,
    pub instructions: Vec<Instruction>,
}

pub struct NativeFunctionDef {
    pub arity: usize,
    pub code: Box<Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken> + 'static>,
}

pub enum FunctionDef {
    Bytecode(BytecodeFunctionDef),
    Native(NativeFunctionDef),
}

impl FunctionDef {
    fn into_function(self, scope: Scope) -> Function {
        match self {
            FunctionDef::Bytecode(def) => Function::from_code(
                Code::create(def.instructions),
                def.arity,
                scope,
            ),
            FunctionDef::Native(def) => Function::from_boxed_fn(def.code, def.arity, scope),
        }
    }
}

pub struct ModuleDef {
    pub name: String,
    pub types: HashMap<String, TypeDef>,
    pub free_functions: HashMap<String, FunctionDef>,
}

impl ModuleDef {
    pub fn register(self, interpreter: &mut Interpreter) {
        let name = self.name.clone();
        interpreter.create_module(&name, move |interpreter, module| {
            for (name, tydef) in self.types {
                module.create_type(interpreter, &name, move |_interpreter, _module, ty| {
                    for (name, funcdef) in tydef.methods {
                        let scope = ty.scope().dup();
                        ty.register_method(&name, funcdef.into_function(scope));
                    }
                });
            }
            for (name, funcdef) in self.free_functions {
                let globals = module.globals.dup();
                module.globals.assign_member(
                    name,
                    function_object_from_function(funcdef.into_function(globals)),
                    interpreter,
                );
            }
        });
    }
}
