use ::*;
use std::mem;

unsafe fn get_int(obj: &Object) -> i64 {
    let ptr: *const i64 = mem::transmute(obj.data.as_ptr());
    *ptr
}

unsafe fn put_int(obj: &mut Object, int: i64) {
    let ptr: *mut i64 = mem::transmute(obj.data.as_mut_ptr());
    *ptr = int;
}

pub fn register_int_type(interpreter: &mut Interpreter) {
    let mut int_ty = Type {
        name: "Int".to_owned(),
        methods: HashMap::new(),
    };

    int_ty.methods.insert(
        interpreter_consts::INIT_METHOD_NAME.to_owned(),
        Method {
            arity: 0,
            code: Code(Rc::new(move |itrp, args| {
                let mut target = args[0].obj_mut();
                target.data.resize(mem::size_of::<i64>(), 0);
                itrp.get_unit_object()
            })),
        },
    );

    int_ty.methods.insert(
        "add".to_owned(),
        Method {
            arity: 1,
            code: Code(Rc::new(move |itrp, args| {
                let a = args[0].obj();
                let b = args[1].obj();

                assert_eq!(a.type_, b.type_);

                let res_obj = itrp.create_object(a.type_);
                unsafe {
                    let mut res_ = res_obj.obj_mut();
                    let (int_a, int_b) = (get_int(&a), get_int(&b));
                    put_int(&mut res_, int_a + int_b);
                }

                res_obj
            })),
        },
    );

}
