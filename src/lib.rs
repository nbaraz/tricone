use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::ops::Range;

enum ErrorKind {
    IndexError,
}

struct TriconeError {
    kind: ErrorKind,
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {

}

#[derive(Debug)]
pub struct SharedSlice {
    data: Rc<RefCell<Vec<u8>>>,
    bounds: Range<usize>,
}

impl SharedSlice {
    fn slice(&self, range: Range<usize>) -> SharedSlice {
        let absolute_range = (range.start + self.bounds.start)..(range.end + self.bounds.start);
        if absolute_range.end > self.bounds.end {
            panic!("Attempted to slice beyond bounds");
        }
        SharedSlice {
            data: Rc::clone(&self.data),
            bounds: absolute_range,
        }
    }
}

#[derive(Debug)]
pub struct PackedObject {
    type_: TypeIndex,
    data: SharedSlice,
}

impl PackedObject {
    fn get_member(&self, interpreter: &Interpreter, idx: usize) -> PackedObject {
        let ty = interpreter.get_type(self.type_);
        let te = &ty.composition[idx];

        match te.embedding_kind {
            EmbeddingKind::Embedded => {
                return PackedObject {
                    type_: te.type_,
                    data: self.data.slice(ty.member_data_range(interpreter, idx)),
                }
            }
            _ => panic!(""),
        }
    }
}

pub struct Method;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmbeddingKind {
    Embedded,
    Ref,
}

#[derive(Debug, Clone, Copy)]
pub struct TypeIndex(ModuleIndex, usize);
pub struct TypeEmbedding {
    type_: TypeIndex,
    embedding_kind: EmbeddingKind,
}

impl TypeEmbedding {
    fn total_size(&self, interpreter: &Interpreter) -> Option<usize> {
        if let EmbeddingKind::Embedded = self.embedding_kind {
            Some(interpreter.get_type(self.type_).total_size(interpreter))
        } else {
            None
        }
    }
}

pub struct Type {
    methods: HashMap<String, Method>,
    composition: Vec<TypeEmbedding>,
    required_size: usize,
}

impl Type {
    fn total_size(&self, interpreter: &Interpreter) -> usize {
        return self.required_size
            + self.composition
                .iter()
                .map(|te| te.total_size(interpreter).unwrap_or(0))
                .sum::<usize>();
    }

    fn num_refs(&self) -> usize {
        self.composition
            .iter()
            .filter(|te| te.embedding_kind != EmbeddingKind::Embedded)
            .count()
    }

    fn member_data_range(&self, interpreter: &Interpreter, idx: usize) -> Range<usize> {
        let (offset, size) = self.composition[..idx]
            .iter()
            .map(|te| te.total_size(interpreter))
            .fold((0, None), |acc, size| (acc.0 + acc.1.unwrap_or(0), size));
        assert!(size.is_some());

        (offset..size.expect("Should only be called for embedded members... This is a bug"))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleIndex(usize);
pub struct Module {
    globals: HashMap<String, PackedObject>,
    types: Vec<Type>,
}

pub struct Interpreter {
    modules: Vec<Module>,
}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {
            modules: vec![],
        }
    }

    pub fn run<I: IntoIterator<Item = Instruction>>(&mut self, instructions: I) {
        println!("Welcome!");
        for insn in instructions {}
    }

    fn get_module(&self, idx: ModuleIndex) -> &Module {
        return &self.modules[idx.0];
    }

    fn get_type(&self, idx: TypeIndex) -> &Type {
        let TypeIndex(modidx, tyidx) = idx;
        &self.get_module(modidx).types[tyidx]
    }

    fn create_packed_object(&self, tyidx: TypeIndex) -> PackedObject {
        let ty = self.get_type(tyidx);
        let total_size = ty.total_size(self);
        let num_refs = ty.num_refs();

        PackedObject {
            type_: tyidx,
            data: SharedSlice {
                data: Rc::new(RefCell::new(vec![0; total_size])),
                bounds: 0..ty.required_size,
            },
        }
    }
}
