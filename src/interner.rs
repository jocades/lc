use std::{collections::HashMap, mem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

pub struct Interner {
    map: HashMap<&'static str, Symbol>,
    vec: Vec<&'static str>,
    buf: String,
    full: Vec<String>,
}

impl Interner {
    pub fn with_capacity(cap: usize) -> Self {
        let cap = cap.next_power_of_two();
        Interner {
            map: HashMap::new(),
            vec: Vec::new(),
            buf: String::with_capacity(cap),
            full: Vec::new(),
        }
    }

    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(&id) = self.map.get(name) {
            return id;
        }

        let name = self.alloc(name);
        let sym = Symbol(self.map.len() as u32);
        self.map.insert(name, sym);
        self.vec.push(name);

        sym
    }

    pub fn lookup(&self, sym: Symbol) -> &str {
        self.vec[sym.0 as usize]
    }

    fn alloc(&mut self, name: &str) -> &'static str {
        let cap = self.buf.capacity();
        if cap < self.buf.len() + name.len() {
            let new_cap = (cap.max(name.len()) + 1).next_power_of_two();
            let new_buf = String::with_capacity(new_cap);
            let old_buf = mem::replace(&mut self.buf, new_buf);
            self.full.push(old_buf);
        }

        let interned = {
            let start = self.buf.len();
            self.buf.push_str(name);
            &self.buf[start..]
        };

        unsafe { &*(interned as *const str) }
    }
}
