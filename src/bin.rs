extern crate tricone;

fn main() {
    let interpreter = tricone::Interpreter::new();
    tricone::do_hello(interpreter);
}
