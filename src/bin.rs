extern crate tricone;

fn main() {
    let mut interpreter = tricone::Interpreter::new();
    tricone::do_hello(&mut interpreter);
}
