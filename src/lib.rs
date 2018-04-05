extern crate arrayvec;

pub mod function;
pub mod interpreter;
#[macro_use]
pub mod generic;
pub mod hello;
pub mod int;
pub mod string;

pub use interpreter::Interpreter;
