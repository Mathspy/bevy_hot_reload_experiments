use std::{fs, io::Read, time::SystemTime};

use libloading::Library;
use object::{Object, ObjectSymbol, Symbol};
use rustc_demangle::demangle;

const LIBRARY_PATH: &str = "../one_fn/target/release/libone_fn.dylib";

fn symbol_bytes<'a>(sym: &'a Symbol) -> &'a [u8] {
    // For some reason the underscore at the start of symbols shouldn't be there, some weird linking
    // shenanigans that I don't truly understand!
    &sym.name_bytes().unwrap()[1..]
}

struct State {
    modified: SystemTime,
    function: fn() -> (),
}

fn load_symbol(buf: &[u8]) -> fn() -> () {
    let object = object::File::parse(buf).unwrap();

    let symbol = object
        .symbols()
        .find(|symbol| {
            demangle(symbol.name().unwrap())
                .to_string()
                .starts_with("one_fn")
        })
        .unwrap();

    let lib = unsafe { Library::new(LIBRARY_PATH) }.unwrap();
    // For now we will leak the library because we don't want to close it on drop.
    let lib = Box::leak(Box::new(lib));
    // This is where future problems will creep up, how do we detect signature change?
    let function = unsafe { lib.get::<fn() -> ()>(symbol_bytes(&symbol)) }.unwrap();

    *function
}

fn main() {
    let mut buf = String::new();
    let mut data = Vec::new();

    let mut file = fs::File::open(LIBRARY_PATH).unwrap();
    let modified = file.metadata().unwrap().modified().unwrap();

    file.read_to_end(&mut data).unwrap();

    let mut state = State {
        modified,
        function: load_symbol(&data),
    };

    loop {
        std::io::stdin().read_line(&mut buf).unwrap();
        buf.clear();
        data.clear();

        let mut file = fs::File::open(LIBRARY_PATH).unwrap();
        let modified = file.metadata().unwrap().modified().unwrap();

        if state.modified < modified {
            file.read_to_end(&mut data).unwrap();

            state = State {
                modified,
                function: load_symbol(&data),
            };
        }

        (state.function)();
    }
}
