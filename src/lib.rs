use std::collections::HashMap;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

#[derive(Debug, Clone)]
enum ErrorKind {
    IndexError,
    WrongArgumentCount,
}

#[derive(Debug, Clone)]
struct TriconeError {
    kind: ErrorKind,
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
}

pub struct Code(Rc<Fn(&mut Interpreter, &[SObject]) -> SObject>);

pub struct Method {
    code: Code,
    arity: usize,
}

impl Method {
    fn dup(&self) -> Method {
        Method {
            code: Code(Rc::clone(&self.code.0)),
            arity: self.arity,
        }
    }

    fn call(
        &self,
        interpreter: &mut Interpreter,
        args: &[SObject],
    ) -> Result<SObject, TriconeError> {
        if self.arity + 1 == args.len() {
            Ok((self.code.0)(interpreter, args))
        } else {
            Err(TriconeError {
                kind: ErrorKind::WrongArgumentCount,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeIndex(ModuleIndex, usize);

pub struct Type {
    name: String,
    methods: HashMap<String, Method>,
}

impl Type {
    fn get_method(&self, name: &str) -> Option<Method> {
        self.methods.get(name).map(Method::dup)
    }
}

pub struct SObject(Rc<RefCell<Object>>);

impl SObject {
    fn new(obj: Object) -> SObject {
        SObject(Rc::new(RefCell::new(obj)))
    }

    fn obj(&self) -> Ref<Object> {
        self.0.borrow()
    }

    fn obj_mut(&self) -> RefMut<Object> {
        self.0.borrow_mut()
    }

    fn dup(&self) -> SObject {
        SObject(Rc::clone(&self.0))
    }
}

pub struct Object {
    members: HashMap<String, SObject>,
    type_: TypeIndex,
    data: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleIndex(pub usize);
pub struct Module {
    globals: SObject,
    types: Vec<Type>,
}

impl Module {
    fn lookup_type_mut(&mut self, name: &str) -> Option<&mut Type> {
        self.types.iter_mut().filter(|ty| ty.name == name).next()
    }

    fn lookup_type_index(&self, name: &str) -> Option<usize> {
        self.types
            .iter()
            .enumerate()
            .filter_map(|(i, ty)| if ty.name == name { Some(i) } else { None })
            .next()
    }
}

struct Scope {
    vars: SObject,
}

struct Thread {
    operation_stack: Vec<SObject>,
    scope_stack: Vec<Scope>,
}

pub struct Interpreter {
    modules: Vec<Module>,
    thread: Thread,
}

pub(crate) mod interpreter_consts {
    // modules depend on objects and vice-versa, need to bootstrap the core module
    pub const CORE_MODULE_ID: ::ModuleIndex = ::ModuleIndex(0);
    pub const SCOPE_TYPE_ID: ::TypeIndex = ::TypeIndex(CORE_MODULE_ID, 0);
    pub const UNIT_TYPE_ID: ::TypeIndex = ::TypeIndex(CORE_MODULE_ID, 1);

    pub const INIT_METHOD_NAME: &'static str = "create";
}

impl Interpreter {
    pub fn new() -> Interpreter {
        let core_module = Module {
            globals: SObject::new(Object {
                members: HashMap::new(),
                type_: interpreter_consts::SCOPE_TYPE_ID,
                data: vec![],
            }),
            types: vec![
                Type {
                    name: "Scope".to_owned(),
                    methods: HashMap::new(),
                },
                Type {
                    name: "Unit".to_owned(),
                    methods: HashMap::new(),
                },
            ],
        };

        let mut interpreter = Interpreter {
            modules: vec![core_module],
            thread: Thread {
                operation_stack: vec![],
                scope_stack: vec![],
            },
        };

        int::register_int_type(&mut interpreter);

        interpreter
    }

    fn get_module(&self, idx: ModuleIndex) -> &Module {
        return &self.modules[idx.0];
    }

    fn get_module_mut(&mut self, idx: ModuleIndex) -> &mut Module {
        return &mut self.modules[idx.0];
    }

    fn lookup_type(&self, modidx: ModuleIndex, name: &str) -> Option<TypeIndex> {
        self.get_module(modidx)
            .lookup_type_index(name)
            .map(|idx| TypeIndex(modidx, idx))
    }

    fn get_type(&self, idx: TypeIndex) -> &Type {
        let TypeIndex(modidx, tyidx) = idx;
        &self.get_module(modidx).types[tyidx]
    }

    fn register_type(&mut self, modidx: ModuleIndex, ty: Type) -> TypeIndex {
        let module = self.get_module_mut(modidx);
        module.types.push(ty);
        TypeIndex(modidx, module.types.len() - 1)
    }

    fn create_object(&mut self, tyidx: TypeIndex) -> SObject {
        let obj = SObject::new(Object {
            members: HashMap::new(),
            type_: tyidx,
            data: vec![],
        });

        if let Some(method) = self.get_type(tyidx)
            .get_method(interpreter_consts::INIT_METHOD_NAME)
        {
            let res = method.call(self, &[obj.dup()]).unwrap();
            assert_eq!(interpreter_consts::UNIT_TYPE_ID, res.obj().type_);
        }

        obj
    }

    fn get_unit_object(&mut self) -> SObject {
        self.create_object(interpreter_consts::UNIT_TYPE_ID)
    }

    fn get_method(&self, obj: &Object, name: &str) -> Option<Method> {
        self.get_type(obj.type_).get_method(name)
    }

    fn call_method(&mut self, name: &str, args: &[SObject]) -> SObject {
        assert!(args.len() >= 1);
        let target = args.last().unwrap();
        let method = self.get_method(&target.obj(), name)
            .expect("Called nonexistent method. TODO: runtime error");
        method.call(self, &args).unwrap()
    }

    fn run_instruction(&mut self, insn: &Instruction) -> SObject {
        use Instruction::*;
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
        }
    }

    fn create_code(&self, instructions: Vec<Instruction>) -> Code {
        Code(Rc::new(move |interpreter, _args| {
            let mut prev = None;
            let scope = interpreter.create_object(interpreter_consts::SCOPE_TYPE_ID);
            interpreter.thread.scope_stack.push(Scope {
                vars: scope,
            });
            for insn in instructions.iter() {
                if let Some(res) = prev {
                    interpreter.thread.operation_stack.push(res)
                }
                prev = Some(interpreter.run_instruction(insn));
            }
            interpreter.thread.scope_stack.pop();
            prev.unwrap_or(interpreter.get_unit_object())
        }))
    }
}

pub mod hello;
mod int;
mod generic;
