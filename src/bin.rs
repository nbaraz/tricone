extern crate tricone;

fn main() {
    let mut interpreter = tricone::Interpreter::new();
    tricone::hello::do_hello(&mut interpreter);
}
