use ::*;

fn register_hello(interpreter: &mut Interpreter) -> TypeIndex {
    let mut hello_ty = Type {
        name: "Hello".to_owned(),
        methods: HashMap::new(),
    };
    hello_ty.methods.insert(
        "hello".to_owned(),
        Method {
            arity: 0,
            code: Code(Rc::new(move |itrp, _args| {
                println!("hello from method!!");
                itrp.get_unit_object()
            })),
        },
    );
    hello_ty.methods.insert(
        interpreter_consts::INIT_METHOD_NAME.to_owned(),
        Method {
            arity: 0,
            code: Code(Rc::new(move |itrp, _args| {
                println!("hello from INIT method!!");
                itrp.get_unit_object()
            })),
        },
    );

    interpreter.register_type(interpreter_consts::CORE_MODULE_ID, hello_ty)
}


pub fn do_hello(interpreter: &mut Interpreter) {
    let hello_idx = register_hello(interpreter);
    assert_eq!(
        Some(hello_idx),
        interpreter.lookup_type(interpreter_consts::CORE_MODULE_ID, "Hello"),
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
    (code.0)(interpreter, &[]);
}
