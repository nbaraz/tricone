use std::collections::HashMap;

use object::{Object, SObject};
use function::{self, Function};
use int;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    IndexError,
    WrongArgumentCount,
}

#[derive(Debug, Clone)]
pub struct TriconeError {
    pub kind: ErrorKind,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    CreateObject {
        type_: TypeIndex,
    },
    Assign {
        // Assign a = pop(), b = pop(), a[name] = `b`
        name: String,
    },
    GetTopScope,
    CallMethod {
        name: String,
        num_args: usize,
    },
    GetMember {
        name: String,
    },
    CallFunctionObject {
        num_args: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeIndex(ModuleIndex, usize);

pub struct Type {
    name: String,
    methods: HashMap<String, Function>,
}

impl Type {
    pub fn new(name: &str) -> Type {
        Type {
            name: name.to_owned(),
            methods: HashMap::new(),
        }
    }

    fn get_method(&self, name: &str) -> Option<Function> {
        self.methods.get(name).map(Function::dup)
    }

    pub fn register_method<F>(&mut self, name: &str, arity: usize, code: F)
    where
        F: Fn(&mut Interpreter, &[SObject]) -> SObject + 'static,
    {
        self.methods
            .insert(name.to_owned(), Function::new(code, arity + 1));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleIndex(pub usize);
pub struct Module {
    globals: SObject,
    types: Vec<Type>,
}

impl Module {
    fn lookup_type_mut(&mut self, name: &str) -> Option<&mut Type> {
        self.types.iter_mut().find(|ty| ty.name == name)
    }

    fn lookup_type_index(&self, name: &str) -> Option<usize> {
        self.types
            .iter()
            .enumerate()
            .filter_map(|(i, ty)| if ty.name == name { Some(i) } else { None })
            .next()
    }
}

pub struct Scope {
    vars: SObject,
}

pub struct Thread {
    operation_stack: Vec<SObject>,
    scope_stack: Vec<Scope>,
}

pub struct Interpreter {
    modules: Vec<Module>,
    thread: Thread,
}

pub(crate) mod consts {
    use interpreter::{ModuleIndex, TypeIndex};
    // modules depend on objects and vice-versa, need to bootstrap the core module
    pub const CORE_MODULE_ID: ModuleIndex = ModuleIndex(0);
    pub const SCOPE_TYPE_ID: TypeIndex = TypeIndex(CORE_MODULE_ID, 0);
    pub const UNIT_TYPE_ID: TypeIndex = TypeIndex(CORE_MODULE_ID, 1);
    pub const FUNCTION_TYPE_ID: TypeIndex = TypeIndex(CORE_MODULE_ID, 2);

    pub const INIT_METHOD_NAME: &str = "create";
}

impl Interpreter {
    pub fn new() -> Interpreter {
        let core_module = Module {
            globals: SObject::new(Object {
                members: HashMap::new(),
                type_: consts::SCOPE_TYPE_ID,
                data: vec![],
            }),
            types: vec![Type::new("Scope"), Type::new("Unit")],
        };

        let mut interpreter = Interpreter {
            modules: vec![core_module],
            thread: Thread {
                operation_stack: vec![],
                scope_stack: vec![],
            },
        };

        let function_tyidx = function::register_func_type(&mut interpreter);
        // The interpreter needs to know if an object is a function object easily
        assert_eq!(function_tyidx, consts::FUNCTION_TYPE_ID);

        int::register_int_type(&mut interpreter);

        interpreter
    }

    pub fn get_module(&self, idx: ModuleIndex) -> &Module {
        &self.modules[idx.0]
    }

    pub fn get_module_mut(&mut self, idx: ModuleIndex) -> &mut Module {
        &mut self.modules[idx.0]
    }

    pub fn lookup_type(&self, modidx: ModuleIndex, name: &str) -> Option<TypeIndex> {
        self.get_module(modidx)
            .lookup_type_index(name)
            .map(|idx| TypeIndex(modidx, idx))
    }

    pub fn get_type(&self, idx: TypeIndex) -> &Type {
        let TypeIndex(modidx, tyidx) = idx;
        &self.get_module(modidx).types[tyidx]
    }

    pub fn register_type(&mut self, modidx: ModuleIndex, ty: Type) -> TypeIndex {
        let module = self.get_module_mut(modidx);
        module.types.push(ty);
        TypeIndex(modidx, module.types.len() - 1)
    }

    pub fn create_object(&mut self, tyidx: TypeIndex) -> SObject {
        let obj = SObject::new(Object {
            members: HashMap::new(),
            type_: tyidx,
            data: vec![],
        });

        if let Some(method) = self.get_type(tyidx).get_method(consts::INIT_METHOD_NAME) {
            let res = method.call(self, &[obj.dup()]).unwrap();
            assert_eq!(consts::UNIT_TYPE_ID, res.obj().type_);
        }

        obj
    }

    pub fn get_unit_object(&mut self) -> SObject {
        self.create_object(consts::UNIT_TYPE_ID)
    }

    fn get_method(&self, obj: &Object, name: &str) -> Option<Function> {
        self.get_type(obj.type_).get_method(name)
    }

    fn call_method(&mut self, name: &str, args: &[SObject]) -> SObject {
        assert!(args.len() >= 1);
        let target = args.last().unwrap();
        let method = self.get_method(&target.obj(), name)
            .expect("Called nonexistent method. TODO: runtime error");
        method.call(self, args).unwrap()
    }

    pub fn create_scope(&mut self) -> Scope {
        Scope {
            vars: self.create_object(consts::SCOPE_TYPE_ID),
        }
    }

    pub fn run_code(&mut self, instructions: &[Instruction]) -> SObject {
        let mut prev = None;
        let scope = self.create_object(consts::SCOPE_TYPE_ID);
        self.thread.scope_stack.push(Scope { vars: scope });
        for insn in instructions.iter() {
            if let Some(res) = prev {
                self.thread.operation_stack.push(res)
            }
            prev = Some(self.run_instruction(insn));
        }
        self.thread.scope_stack.pop();
        prev.unwrap_or_else(|| self.get_unit_object())
    }

    pub fn run_instruction(&mut self, insn: &Instruction) -> SObject {
        use self::Instruction::*;
        match *insn {
            CreateObject { type_ } => self.create_object(type_),
            Assign { ref name } => {
                let mut scope = self.thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 2 items, 0 found");
                let item = self.thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 2 items, only 1 found");
                scope.obj_mut().members.insert(name.clone(), item);
                self.get_unit_object()
            }
            GetTopScope => self.thread
                .scope_stack
                .last()
                .expect("Must have at least one scope")
                .vars
                .dup(),
            CallMethod {
                ref name,
                mut num_args,
            } => {
                num_args += 1;

                if self.thread.operation_stack.len() < num_args {
                    panic!("Not enough arguments passed! TODO: runtime error");
                }

                let op_stack_len = self.thread.operation_stack.len();
                let args = self.thread
                    .operation_stack
                    .split_off(op_stack_len - num_args);

                self.call_method(name, &args)
            }
            GetMember { ref name } => {
                let item = self.thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 1 item, was empty");
                let temp_obj = &item.obj();
                temp_obj
                    .members
                    .get(name)
                    .expect("Requested nonexistent member. TODO: runtime error")
                    .dup()
            }
            CallFunctionObject { num_args } => {
                if self.thread.operation_stack.len() < num_args {
                    panic!("Not enough arguments passed! TODO: runtime error");
                }

                let op_stack_len = self.thread.operation_stack.len();
                let args = self.thread
                    .operation_stack
                    .split_off(op_stack_len - num_args);

                let function_obj = self.thread
                    .operation_stack
                    .pop()
                    .expect("Need a function to call!");
                let function_ref = function_obj.obj();

                // Should be a runtime error
                assert_eq!(function_ref.type_, consts::FUNCTION_TYPE_ID);
                let function = function::function_from_function_object(&function_ref);

                function.call(self, &args).unwrap()
            }
        }
    }
}