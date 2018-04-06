use arrayvec::ArrayVec;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::mem;
use std::ops::Deref;
use std::process::abort;
use std::rc::Rc;

use function::{self, Function};
use int;
use string;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    IndexError,
    WrongArgumentCount,
    TypeError,
}

#[derive(Debug, Clone)]
pub struct TriconeError {
    pub kind: ErrorKind,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    CreateObject {
        type_: TypeIndex,
        num_args: usize,
    },
    Assign {
        // Assign a = pop(), b = pop(), a[name] = `b`
        name: String,
    },
    GetTopScope,
    GetModuleGlobals {
        name: String,
    },
    CallMethod {
        name: String,
        num_args: usize,
        use_result: bool,
    },
    GetMember {
        name: String,
    },
    LookupName {
        name: String,
    },
    CallFunctionObject {
        num_args: usize,
        use_result: bool,
    },
    CreateString {
        value: String,
    },
    CreateInt {
        value: i64,
    },
    Diag,
    DebugPrintObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeIndex(ModuleIndex, usize);

pub struct Type {
    name: String,
    methods: HashMap<String, Function>,
    scope: Scope,
    pub index: TypeIndex,
}

impl Type {
    pub fn new(name: &str, index: TypeIndex) -> Type {
        Type {
            name: name.to_owned(),
            methods: HashMap::new(),
            scope: Scope::new(),
            index,
        }
    }

    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    fn get_method(&self, name: &str) -> Option<Function> {
        self.methods.get(name).map(Function::dup)
    }

    pub fn register_method(&mut self, name: &str, func: Function) {
        self.methods.insert(name.to_owned(), func);
    }

    pub fn register_native_method<F>(&mut self, name: &str, arity: usize, code: F)
    where
        F: Fn(&mut Interpreter, &[ObjectToken]) -> Option<ObjectToken> + 'static,
    {
        assert!(arity >= 1);
        let scope = self.scope.dup();
        self.register_method(name, Function::new(code, arity, scope));
    }

    pub fn register_bytecode_method(
        &mut self,
        name: &str,
        arity: usize,
        instructions: Vec<Instruction>,
    ) {
        assert!(arity >= 1);
        let code = function::Code::create(instructions);
        let scope = self.scope.dup();
        self.register_method(name, Function::from_code(code, arity, scope));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleIndex(pub usize);
pub struct Module {
    pub name: String,
    pub index: ModuleIndex,
    pub types: Vec<Type>,
    pub globals: Scope,
}

impl Module {
    pub fn new(index: ModuleIndex, name: &str) -> Module {
        Module {
            name: name.to_owned(),
            index,
            types: vec![],
            globals: Scope::new(),
        }
    }

    #[allow(unused)]
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

    pub fn create_type<F, O>(
        &mut self,
        interpreter: &mut Interpreter,
        name: &str,
        func: F,
    ) -> (TypeIndex, O)
    where
        F: FnOnce(&mut Interpreter, &mut Module, &mut Type) -> O,
    {
        let index = TypeIndex(self.index, self.types.len());
        let mut ty = Type::new(name, index);
        let res = (func)(interpreter, self, &mut ty);
        self.types.push(ty);
        (index, res)
    }
}

pub struct Scope {
    pub vars: ObjectToken,
}

macro_rules! get_internal_member {
    ($target:expr, $name:expr) => {
        $target.get_member(concat!("!", $name))
    };
}

macro_rules! assign_member_internal {
    ($target:expr, $name:expr, $val:expr, $interpreter:expr) => {
        $target.assign_member(concat!("!", $name).to_owned(), $val, $interpreter)
    };
}

macro_rules! with_internal_member {
    ($target:expr, $name:expr, $func:expr) => {
        $target.with_member_ref(concat!("!", $name), $func)
    };
}

impl Scope {
    pub fn new() -> Scope {
        Scope {
            vars: ObjectToken::new(Object::raw_new(consts::SCOPE_TYPE_ID)),
        }
    }

    pub fn dup(&self) -> Scope {
        Scope {
            vars: self.vars.dup(),
        }
    }

    fn into_child(self, interpreter: &mut Interpreter) -> Scope {
        let child = Scope::new();
        assign_member_internal!(child.vars, "parent", self.vars, interpreter);
        child
    }

    fn from_object(obj: ObjectToken) -> Result<Scope, TriconeError> {
        if obj.obj().type_ == consts::SCOPE_TYPE_ID {
            Ok(Scope { vars: obj })
        } else {
            Err(TriconeError {
                kind: ErrorKind::TypeError,
            })
        }
    }

    fn parent(&self) -> Option<Scope> {
        get_internal_member!(self.vars, "parent").map(|obj| Scope::from_object(obj).unwrap())
    }

    fn with_parent<F, O>(vars: &ObjectToken, func: F) -> O
    where
        F: FnOnce(Option<&ObjectToken>) -> O,
    {
        with_internal_member!(vars, "parent", func)
    }

    fn token_lookup_name(vars: &ObjectToken, name: &str) -> Option<ObjectToken> {
        println!(
            "looking for {} in {:?}",
            name,
            vars.obj().members.keys().collect::<Vec<_>>()
        );
        let opt = vars.get_member(name);
        opt.or_else(|| {
            Scope::with_parent(vars, |parent| {
                parent.and_then(|p| Scope::token_lookup_name(p, name))
            })
        })
    }

    fn lookup_name(&self, name: &str) -> Option<ObjectToken> {
        let res = Scope::token_lookup_name(&self.vars, name);
        if res.is_some() {
            println!("found {}!", name);
        } else {
            println!("did not find {}!", name);
        }
        res
    }
}

impl Default for Scope {
    fn default() -> Scope {
        Scope::new()
    }
}

impl Deref for Scope {
    type Target = ObjectToken;

    fn deref(&self) -> &ObjectToken {
        &self.vars
    }
}

pub struct Frame {
    top_scope: Scope,
}

impl Frame {
    fn new(top_scope: Scope) -> Frame {
        Frame { top_scope }
    }

    fn push_scope(&mut self, interpreter: &mut Interpreter) {
        unsafe {
            let scope = mem::replace(&mut self.top_scope, mem::uninitialized());
            let uninitialized = mem::replace(&mut self.top_scope, scope.into_child(interpreter));
            mem::forget(uninitialized);
        }
    }

    fn pop_scope(&mut self, interpreter: &mut Interpreter) {
        let mut temp = self.top_scope.parent().unwrap();
        mem::swap(&mut temp, &mut self.top_scope);
        interpreter.drop_token(temp.vars);
    }
}

impl Deref for Frame {
    type Target = Scope;
    fn deref(&self) -> &Scope {
        &self.top_scope
    }
}

pub struct Thread {
    operation_stack: Vec<ObjectToken>,
    frame_stack: Vec<Frame>,
}

impl Thread {
    fn top_frame(&mut self) -> &mut Frame {
        self.frame_stack.last_mut().unwrap()
    }
}

pub struct ObjectToken(Rc<RefCell<Object>>);

impl ObjectToken {
    pub fn new(obj: Object) -> ObjectToken {
        ObjectToken(Rc::new(RefCell::new(obj)))
    }

    fn get_member(&self, name: &str) -> Option<ObjectToken> {
        self.obj().members.get(name).map(ObjectToken::dup)
    }

    fn with_member_ref<F, O>(&self, name: &str, func: F) -> O
    where
        F: FnOnce(Option<&ObjectToken>) -> O,
    {
        let obj = self.obj();
        (func)(obj.members.get(name))
    }

    pub fn assign_member(&self, name: String, obj: ObjectToken, interpreter: &mut Interpreter) {
        if let Some(token) = self.obj_mut().members.insert(name, obj) {
            interpreter.drop_token(token);
        }
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

impl PartialEq for ObjectToken {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl Eq for ObjectToken {}

impl Hash for ObjectToken {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state)
    }
}

thread_local! {
    static OBJECTS_BEING_PRINTED: RefCell<HashSet<*mut Object>> = RefCell::new(HashSet::new());
}

impl fmt::Debug for ObjectToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        OBJECTS_BEING_PRINTED.with(|set| {
            let existed = { set.borrow_mut().insert(self.0.as_ptr()) };
            if existed {
                let res = if let Ok(obj) = self.0.try_borrow() {
                    fmt::Debug::fmt(&obj, f)
                } else {
                    f.write_str("<Object, borrowed>")
                };
                set.borrow_mut().remove(&self.0.as_ptr());
                res
            } else {
                f.write_str("{...}")
            }
        })
    }
}

impl Drop for ObjectToken {
    fn drop(&mut self) {
        // TODO: abort
        let obj = self.obj();
        println!(
            "Pass object tokens to the interpreter to destroy them, ty: {:?}, members: {:?}",
            obj.type_, obj.members
        );
        abort();
    }
}

pub struct Object {
    pub members: HashMap<String, ObjectToken>,
    pub type_: TypeIndex,
    pub data: Vec<u8>,
}

impl Object {
    pub fn raw_new(type_: TypeIndex) -> Object {
        Object {
            members: HashMap::new(),
            type_,
            data: vec![],
        }
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.debug_struct("Object")
            .field("type", &self.type_)
            .field("values", &self.members)
            .finish()
    }
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
        let mut interpreter = Interpreter {
            modules: vec![],
            thread: Thread {
                operation_stack: vec![],
                frame_stack: vec![],
            },
        };

        interpreter.create_module("core", move |interpreter, module| {
            for ty_name in &["Scope", "Unit"] {
                module.create_type(interpreter, ty_name, move |_interpreter, _module, _ty| {});
            }
            function::register_func_type(interpreter, module);
            int::register_int_type(interpreter, module);
            string::register_string_type(interpreter, module);
        });

        interpreter
    }

    pub fn create_module<F, O>(&mut self, name: &str, func: F) -> (ModuleIndex, O)
    where
        F: FnOnce(&mut Interpreter, &mut Module) -> O,
    {
        let mod_idx = ModuleIndex(self.modules.len());
        let mut module = Module::new(mod_idx, name);
        let res = (func)(self, &mut module);
        self.modules.push(module);
        (mod_idx, res)
    }

    fn lookup_module_index(&self, name: &str) -> Option<ModuleIndex> {
        self.modules
            .iter()
            .enumerate()
            .find(|(_, m)| m.name == name)
            .map(|(i, _)| ModuleIndex(i))
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

    pub fn with_new_frame<F, O>(&mut self, scope: Scope, function: F) -> O
    where
        F: FnOnce(&mut Interpreter) -> O,
    {
        let mut frame = Frame::new(scope);
        frame.push_scope(self);
        self.thread.frame_stack.push(frame);
        let res = (function)(self);
        let mut frame = self.thread.frame_stack.pop().unwrap();
        frame.pop_scope(self);
        self.drop_token(frame.top_scope.vars);
        res
    }

    fn with_current_frame<F, O>(&mut self, function: F) -> O
    where
        F: FnOnce(&mut Interpreter, &mut Frame) -> O,
    {
        // Ugly :(
        let mut frame = self.thread.frame_stack.pop().unwrap();
        let res = (function)(self, &mut frame);
        self.thread.frame_stack.push(frame);
        res
    }

    pub fn with_new_scope<F, O>(&mut self, function: F) -> O
    where
        F: FnOnce(&mut Interpreter) -> O,
    {
        self.with_current_frame(|i, f| f.push_scope(i));
        let res = (function)(self);
        self.with_current_frame(|i, f| f.pop_scope(i));
        res
    }

    pub fn create_object(&mut self, tyidx: TypeIndex, num_args: usize) -> ObjectToken {
        let obj = ObjectToken::new(Object {
            members: HashMap::new(),
            type_: tyidx,
            data: vec![],
        });

        if let Some(create) = self.get_type(tyidx).get_method(consts::CREATE_METHOD_NAME) {
            let mut args = Vec::with_capacity(num_args);
            let op_stack_len = self.thread.operation_stack.len();
            args.push(obj.dup());
            args.extend(
                self.thread
                    .operation_stack
                    .drain((op_stack_len - num_args)..op_stack_len),
            );
            let res = self.call_function_with_owned_args(create, args);
            self.drop_unit(res);
        }

        obj
    }

    fn call_function_with_owned_args<Args>(
        &mut self,
        func: Function,
        args: Args,
    ) -> Option<ObjectToken>
    where
        Args: IntoIterator<Item = ObjectToken> + AsRef<[ObjectToken]>,
    {
        let res = func.call(self, args.as_ref());
        for arg in args {
            self.drop_token(arg);
        }
        self.drop_token(func.closure.vars);
        res.unwrap()
    }

    fn drop_unit(&mut self, unit: Option<ObjectToken>) {
        if let Some(obj) = unit {
            assert_eq!(consts::UNIT_TYPE_ID, obj.obj().type_);
            self.drop_token(obj);
        }
    }

    fn maybe_call_no_args_no_ret_method(&mut self, token: &ObjectToken, name: &str) {
        let tyidx = token.obj().type_;

        if let Some(method) = self.get_type(tyidx).get_method(name) {
            let args = ArrayVec::from([token.dup()]);
            let res = method.call(self, &args).unwrap();
            for arg in args {
                self.drop_token(arg);
            }
            if let Some(obj) = res {
                assert_eq!(consts::UNIT_TYPE_ID, obj.obj().type_);
                self.drop_token(obj);
            }
            self.drop_token(method.closure.vars);
        }
    }

    pub fn get_unit_object(&mut self) -> ObjectToken {
        self.create_object(consts::UNIT_TYPE_ID, 0)
    }

    fn get_method(&self, obj: &Object, name: &str) -> Option<Function> {
        self.get_type(obj.type_).get_method(name)
    }

    fn call_method(&mut self, name: &str, args: &[ObjectToken]) -> Option<ObjectToken> {
        assert!(args.len() >= 1);
        let target = args.last().unwrap();
        let method = self.get_method(&target.obj(), name)
            .expect("Called nonexistent method. TODO: runtime error");
        let res = method.call(self, args).unwrap();
        self.drop_token(method.closure.vars);
        res
    }

    pub fn create_scope(&mut self) -> Scope {
        Scope {
            vars: self.create_object(consts::SCOPE_TYPE_ID, 0),
        }
    }

    pub fn run_code(&mut self, instructions: &[Instruction]) -> Option<ObjectToken> {
        let mut prev = None;
        for insn in instructions.iter() {
            if let Some(res) = prev {
                self.thread.operation_stack.push(res)
            }
            prev = self.run_instruction(insn);
        }
        prev
    }

    pub fn drop_token(&mut self, token: ObjectToken) {
        if Rc::strong_count(&token.0) == 1 {
            if token.obj().type_ != consts::UNIT_TYPE_ID {
                self.maybe_call_no_args_no_ret_method(&token, consts::DROP_METHOD_NAME);
                assert_eq!(Rc::strong_count(&token.0), 1);
            }

            let mut object = Rc::try_unwrap(token.into_rc()).unwrap().into_inner();

            for (_, obj) in object.members.drain() {
                self.drop_token(obj);
            }
        } else {
            // will drop normally
            token.into_rc();
        }
    }

    fn get_args_from_stack<O>(&mut self, num_args: usize, container: &mut O)
    where
        O: Extend<ObjectToken>,
    {
        let op_stack_len = self.thread.operation_stack.len();
        container.extend(
            self.thread
                .operation_stack
                .drain((op_stack_len - num_args)..op_stack_len),
        )
    }

    pub fn run_instruction(&mut self, insn: &Instruction) -> Option<ObjectToken> {
        println!("running {:?}", insn);

        use self::Instruction::*;
        match *insn {
            CreateObject { type_, num_args } => Some(self.create_object(type_, num_args)),
            Assign { ref name } => {
                let mut scope = self.thread
                    .frame_stack
                    .last()
                    .expect("Must have at least one scope")
                    .vars
                    .dup();
                let item = self.thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 2 items, only 1 found");
                scope.assign_member(name.clone(), item, self);
                self.drop_token(scope);
                None
            }
            GetTopScope => Some(
                self.thread
                    .frame_stack
                    .last()
                    .expect("Must have at least one scope")
                    .vars
                    .dup(),
            ),
            GetModuleGlobals { ref name } => {
                let idx = self.lookup_module_index(name)
                    .expect("Module does not exist! TODO: runtime error");
                Some(self.get_module(idx).globals.vars.dup())
            }
            CallMethod {
                ref name,
                mut num_args,
                use_result,
            } => {
                num_args += 1;

                if self.thread.operation_stack.len() < num_args {
                    panic!("Not enough arguments passed! TODO: runtime error");
                }

                let mut args = Vec::with_capacity(num_args);
                self.get_args_from_stack(num_args, &mut args);
                let res = self.call_method(name, &args);
                for arg in args {
                    self.drop_token(arg);
                }
                if use_result {
                    Some(res.unwrap_or_else(|| self.get_unit_object()))
                } else {
                    if let Some(obj) = res {
                        self.drop_token(obj);
                    }
                    None
                }
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
            LookupName { ref name } => self.thread
                .top_frame()
                .lookup_name(name)
                .or_else(|| panic!("Object does not exist! TODO: runtime error")),
            CallFunctionObject {
                num_args,
                use_result,
            } => {
                if self.thread.operation_stack.len() < num_args {
                    panic!("Not enough arguments passed! TODO: runtime error");
                }

                let mut args = Vec::with_capacity(num_args);
                self.get_args_from_stack(num_args, &mut args);
                let function_obj = self.thread
                    .operation_stack
                    .pop()
                    .expect("Need a function to call!");
                let res = {
                    let function_ref = function_obj.obj();

                    // Should be a runtime error
                    if function_ref.type_ != consts::FUNCTION_TYPE_ID {
                        panic!(
                            "Expected function type ({:?}), got: {:?}",
                            consts::FUNCTION_TYPE_ID,
                            function_ref
                        );
                    }
                    let function = function::function_from_function_object(&function_ref);

                    function.call(self, &args).unwrap()
                };

                for arg in args {
                    self.drop_token(arg);
                }
                self.drop_token(function_obj);

                if use_result {
                    Some(res.unwrap_or_else(|| self.get_unit_object()))
                } else {
                    if let Some(obj) = res {
                        self.drop_token(obj);
                    }
                    None
                }
            }
            CreateString { ref value } => Some(string::create_string(self, value.clone())),
            CreateInt { value } => Some(int::create_int(self, value)),
            Diag => {
                println!("{:?}", self.thread.operation_stack);
                None
            }
            DebugPrintObject => {
                let item = self.thread
                    .operation_stack
                    .pop()
                    .expect("Stack needs 1 item, was empty");
                println!("{:?}", &item);
                self.drop_token(item);
                None
            }
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter::new()
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        let unit = self.get_unit_object();
        let mut scopes = vec![];

        for module in &mut self.modules {
            scopes.push(mem::replace(
                &mut module.globals,
                Scope { vars: unit.dup() },
            ));
            for ty in &mut module.types {
                scopes.push(mem::replace(&mut ty.scope, Scope { vars: unit.dup() }));

                for method in ty.methods.values_mut() {
                    scopes.push(mem::replace(
                        &mut method.closure,
                        Scope { vars: unit.dup() },
                    ));
                }
            }
        }

        for scope in scopes {
            self.drop_token(scope.vars);
        }

        self.drop_token(unit);

        let modules = mem::replace(&mut self.modules, Vec::new());
        for module in modules {
            for ty in module.types {
                for (_, method) in ty.methods {
                    self.drop_token(method.closure.vars);
                }
                self.drop_token(ty.scope.vars);
            }
            self.drop_token(module.globals.vars);
        }
    }
}
