use rustc_data_structures::fx::FxHashSet;
use rustc_span::Symbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Var {
    id: Symbol,
}
pub static mut VARS: Option<FxHashSet<u32>> = None;
pub static mut UNIFS: Option<FxHashSet<(i32, i32)>> = None;

impl Var {
    pub fn arg(id: Symbol) -> Self {
        Self { id }
    }
    pub fn new(name: &str) -> Self {
        let id = Symbol::intern(name);
        Self { id }
    }
    pub fn extend(&self, field: &str) -> Self {
        Self::new(&(self.uuid() + "." + field))
    }
    pub fn uuid(&self) -> String {
        self.id.as_str().to_string()
    }
    pub fn rname(&self) -> String {
        self.uuid()
            .chars()
            .map(|x| match x {
                '.' => '_',
                '*' => 'v',
                '^' => 'f',
                _ => x,
            })
            .collect()
    }
}
