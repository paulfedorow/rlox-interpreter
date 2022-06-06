use std::cell::RefCell;
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use string_interner::StringInterner;

pub type Symbol = SymbolU32;

pub struct Interner {
    pub sym_this: Symbol,
    pub sym_init: Symbol,
    pub sym_super: Symbol,
    interner: RefCell<StringInterner<StringBackend<Symbol>>>,
}

impl Interner {
    pub fn new() -> Interner {
        let mut interner = StringInterner::<StringBackend<Symbol>>::new();
        Interner {
            sym_this: interner.get_or_intern("this"),
            sym_init: interner.get_or_intern("init"),
            sym_super: interner.get_or_intern("super"),
            interner: RefCell::new(interner),
        }
    }

    pub fn resolve(&self, symbol: Symbol) -> String {
        String::from(
            self.interner
                .borrow()
                .resolve(symbol)
                .expect("Resolving an invalid symbol"),
        )
    }

    pub fn get_or_intern<T>(&self, string: T) -> Symbol
    where
        T: AsRef<str>,
    {
        self.interner.borrow_mut().get_or_intern(string)
    }
}
