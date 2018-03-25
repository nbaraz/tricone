use std::collections::HashMap;
use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

enum ErrorKind {
    IndexError,
}

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

pub struct Code(Rc<Fn(&[SObject]) -> SObject>);

pub struct Method {
    code: Code,
    arity: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeIndex(ModuleIndex, usize);

pub struct Type {
    name: String,
    methods: HashMap<String, Method>,
}

pub struct SObject(Rc<RefCell<Object>>);

impl SObject {
    fn new(obj: Object) -> SObject {
        SObject(Rc::new(RefCell::new(obj)))
    }

    fn obj(&self) -> Ref<Object> {
        self.0.borrow()
    }

    fn obj_mut(&mut self) -> RefMut<Object> {
        self.0.borrow_mut()
    }

    fn dup(&self) -> SObject {
        SObject(Rc::clone(&self.0))
    }
}

pub struct Object {
    members: HashMap<String, SObject>,
    type_: TypeIndex,
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

mod interpreter_consts {
    // modules depend on objects and vice-versa, need to bootstrap the core module
    pub const CORE_MODULE_ID: ::ModuleIndex = ::ModuleIndex(0);
    pub const SCOPE_TYPE_ID: ::TypeIndex = ::TypeIndex(CORE_MODULE_ID, 0);
    pub const UNIT_TYPE_ID: ::TypeIndex = ::TypeIndex(CORE_MODULE_ID, 1);
}

impl Interpreter {
    pub fn new() -> Rc<RefCell<Interpreter>> {
        let core_module = Module {
            globals: SObject::new(Object {
                members: HashMap::new(),
                type_: interpreter_consts::SCOPE_TYPE_ID,
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

        let interpreter = Rc::new(RefCell::new(Interpreter {
            modules: vec![core_module],
            thread: Thread {
                operation_stack: vec![],
                scope_stack: vec![],
            },
        }));

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

    fn create_object(&self, tyidx: TypeIndex) -> Object {
        let _ = self.get_type(tyidx);

        Object {
            members: HashMap::new(),
            type_: tyidx,
        }
    }

    fn get_unit_object(&self) -> SObject {
        SObject::new(self.create_object(interpreter_consts::UNIT_TYPE_ID))
    }

    fn run_instruction(interpreter: &RefCell<Interpreter>, insn: &Instruction) -> SObject {
        use Instruction::*;
        match *insn {
            CreateObject { type_ } => SObject::new(interpreter.borrow().create_object(type_)),
            Assign { ref name } => {
                let mut self_ = interpreter.borrow_mut();
                let mut scope = self_
                    .thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 2 items, 0 found");
                let item = self_
                    .thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 2 items, only 1 found");
                scope.obj_mut().members.insert(name.clone(), item);
                self_.get_unit_object()
            }
            GetTopScope => interpreter
                .borrow()
                .thread
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
                let op_stack_len;
                let (code, args) = {
                    let mut self_ = interpreter.borrow_mut();
                    if self_.thread.operation_stack.len() < num_args {
                        panic!("Not enough arguments passed! TODO: runtime error");
                    }
                    let code = {
                        let target = self_.thread.operation_stack.last().unwrap();
                        let method = self_
                            .get_type(target.obj().type_)
                            .methods
                            .get(name)
                            .expect("Called nonexistent method. TODO: runtime error");

                        if method.arity != num_args - 1 {
                            panic!("Wrong number of arguments. TODO: runtime error");
                        }

                        Rc::clone(&method.code.0)
                    };

                    op_stack_len = self_.thread.operation_stack.len();
                    let args = self_
                        .thread
                        .operation_stack
                        .split_off(op_stack_len - num_args);

                    (code, args)
                };
                (code)(&args)
            }
            GetMember { ref name } => {
                let item = interpreter
                    .borrow_mut()
                    .thread
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

    fn create_code(interpreter: Rc<RefCell<Interpreter>>, instructions: Vec<Instruction>) -> Code {
        Code(Rc::new(move |args| {
            let mut prev = None;
            for insn in instructions.iter() {
                if let Some(res) = prev {
                    interpreter.borrow_mut().thread.operation_stack.push(res)
                }
                prev = Some(Interpreter::run_instruction(&*interpreter, insn));
            }
            prev.unwrap_or(interpreter.borrow().get_unit_object())
        }))
    }
}

fn register_hello(interpreter: Rc<RefCell<Interpreter>>) -> TypeIndex {
    let itrp = Rc::clone(&interpreter);

    let mut hello_ty = Type {
        name: "Hello".to_owned(),
        methods: HashMap::new(),
    };
    hello_ty.methods.insert(
        "hello".to_owned(),
        Method {
            arity: 0,
            code: Code(Rc::new(move |args| {
                println!("hello from method!!");
                itrp.borrow().get_unit_object()
            })),
        },
    );

    interpreter
        .borrow_mut()
        .register_type(interpreter_consts::CORE_MODULE_ID, hello_ty)
}

pub fn do_hello(interpreter: Rc<RefCell<Interpreter>>) {
    let hello_idx = register_hello(Rc::clone(&interpreter));
    assert_eq!(
        Some(hello_idx),
        interpreter.borrow().lookup_type(interpreter_consts::CORE_MODULE_ID, "Hello"),
    );

    use Instruction::*;
    let code = Interpreter::create_code(
        interpreter,
        vec![
            CreateObject { type_: hello_idx },
            CallMethod {
                name: "hello".to_owned(),
                num_args: 0,
            },
        ],
    );
    (code.0)(&[]);
}
