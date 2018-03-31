use std::collections::HashMap;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;
use std::mem;
use std::process::abort;
use arrayvec::ArrayVec;

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
        F: Fn(&mut Interpreter, &[ObjectToken]) -> ObjectToken + 'static,
    {
        self.methods
            .insert(name.to_owned(), Function::new(code, arity + 1));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleIndex(pub usize);
pub struct Module {
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
    vars: ObjectToken,
}

pub struct Thread {
    operation_stack: Vec<ObjectToken>,
    scope_stack: Vec<Scope>,
}

#[derive(Debug)]
pub struct ObjectToken(Rc<RefCell<Object>>);

impl ObjectToken {
    pub fn new(obj: Object) -> ObjectToken {
        ObjectToken(Rc::new(RefCell::new(obj)))
    }

    pub fn obj(&self) -> Ref<Object> {
        self.0.borrow()
    }

    pub fn obj_mut(&self) -> RefMut<Object> {
        self.0.borrow_mut()
    }

    pub fn dup(&self) -> ObjectToken {
        ObjectToken(Rc::clone(&self.0))
    }

    fn into_rc(mut self) -> Rc<RefCell<Object>> {
        unsafe {
            let rc = mem::replace(&mut self.0, mem::uninitialized());
            mem::forget(self);
            rc
        }
    }
}

impl Drop for ObjectToken {
    fn drop(&mut self) {
        // TODO: abort
        println!("Pass object tokens to the interpreter to destroy them");
        abort();
    }
}

#[derive(Debug)]
pub struct Object {
    pub members: HashMap<String, ObjectToken>,
    pub type_: TypeIndex,
    pub data: Vec<u8>,
}

pub(crate) mod consts {
    use interpreter::{ModuleIndex, TypeIndex};
    // modules depend on objects and vice-versa, need to bootstrap the core module
    pub const CORE_MODULE_ID: ModuleIndex = ModuleIndex(0);
    pub const SCOPE_TYPE_ID: TypeIndex = TypeIndex(CORE_MODULE_ID, 0);
    pub const UNIT_TYPE_ID: TypeIndex = TypeIndex(CORE_MODULE_ID, 1);
    pub const FUNCTION_TYPE_ID: TypeIndex = TypeIndex(CORE_MODULE_ID, 2);

    pub const CREATE_METHOD_NAME: &str = "create";
    pub const DROP_METHOD_NAME: &str = "drop";
}

pub struct Interpreter {
    modules: Vec<Module>,
    thread: Thread,
}

impl Interpreter {
    pub fn new() -> Interpreter {
        let core_module = Module {
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

    pub fn create_object(&mut self, tyidx: TypeIndex) -> ObjectToken {
        let obj = ObjectToken::new(Object {
            members: HashMap::new(),
            type_: tyidx,
            data: vec![],
        });

        self.maybe_call_no_args_no_ret_method(&obj, consts::CREATE_METHOD_NAME);
        obj
    }

    fn maybe_call_no_args_no_ret_method(&mut self, token: &ObjectToken, name: &str) {
        let tyidx = token.obj().type_;

        if let Some(method) = self.get_type(tyidx).get_method(name) {
            let args = ArrayVec::from([token.dup()]);
            let res = method.call(self, &args).unwrap();
            assert_eq!(consts::UNIT_TYPE_ID, res.obj().type_);
            for arg in args.into_iter() {
                self.drop_token(arg);
            }
            self.drop_token(res);
        }
    }

    pub fn get_unit_object(&mut self) -> ObjectToken {
        self.create_object(consts::UNIT_TYPE_ID)
    }

    fn get_method(&self, obj: &Object, name: &str) -> Option<Function> {
        self.get_type(obj.type_).get_method(name)
    }

    fn call_method(&mut self, name: &str, args: &[ObjectToken]) -> ObjectToken {
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

    pub fn run_code(&mut self, instructions: &[Instruction]) -> ObjectToken {
        let mut prev = None;
        let scope = self.create_object(consts::SCOPE_TYPE_ID);
        self.thread.scope_stack.push(Scope { vars: scope });
        for insn in instructions.iter() {
            if let Some(res) = prev {
                self.thread.operation_stack.push(res)
            }
            prev = self.run_instruction(insn);
        }
        let scope = self.thread.scope_stack.pop().unwrap();
        self.drop_token(scope.vars);
        prev.unwrap_or_else(|| self.get_unit_object())
    }

    pub fn drop_token(&mut self, token: ObjectToken) {
        if Rc::strong_count(&token.0) == 1 {
            self.maybe_call_no_args_no_ret_method(&token, consts::DROP_METHOD_NAME);
            assert_eq!(Rc::strong_count(&token.0), 1);

            let mut object = Rc::try_unwrap(token.into_rc()).unwrap().into_inner();

            for (_, obj) in object.members.drain() {
                self.drop_token(obj);
            }
        } else {
            // will drop normally
            token.into_rc();
        }
    }

    pub fn run_instruction(&mut self, insn: &Instruction) -> Option<ObjectToken> {
        use self::Instruction::*;
        match *insn {
            CreateObject { type_ } => Some(self.create_object(type_)),
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
                None
            }
            GetTopScope => Some(
                self.thread
                    .scope_stack
                    .last()
                    .expect("Must have at least one scope")
                    .vars
                    .dup(),
            ),
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

                let res = self.call_method(name, &args);
                for arg in args {
                    self.drop_token(arg);
                }
                Some(res)
            }
            GetMember { ref name } => {
                let item = self.thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 1 item, was empty");
                let res = {
                    let temp_obj = &item.obj();
                    temp_obj
                        .members
                        .get(name)
                        .expect("Requested nonexistent member. TODO: runtime error")
                        .dup()
                };
                self.drop_token(item);
                Some(res)
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

                let res = function.call(self, &args).unwrap();
                for arg in args {
                    self.drop_token(arg);
                }
                Some(res)
            }
        }
    }
}
