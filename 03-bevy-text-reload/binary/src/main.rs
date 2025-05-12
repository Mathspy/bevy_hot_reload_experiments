use std::{
    any::TypeId,
    fs::File,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use bevy::{
    app::{App, AppExitWithHotReload},
    ecs::schedule::Schedules,
};
use libloading::Library;

const LIBRARY_PATH: &str = "../hello_from_bevy/target/release/libhello_from_bevy.dylib";

struct Symbols {
    create_app: fn() -> App,
    get_hot_reload_detector: fn(&App) -> Arc<AtomicBool>,
    hot_reload_app: fn(App, App) -> App,
}

fn load_symbols() -> Symbols {
    let lib = unsafe { Library::new(LIBRARY_PATH) }.unwrap();
    // For now we will leak the library because we don't want to close it on drop.
    let lib = Box::leak(Box::new(lib));

    let create_app = unsafe { lib.get::<fn() -> App>(b"create_app") }.unwrap();
    let get_hot_reload_detector =
        unsafe { lib.get::<fn(&App) -> Arc<AtomicBool>>(b"get_hot_reload_detector") }.unwrap();
    let hot_reload_app = unsafe { lib.get::<fn(App, App) -> App>(b"hot_reload_app") }.unwrap();

    Symbols {
        create_app: *create_app,
        get_hot_reload_detector: *get_hot_reload_detector,
        hot_reload_app: *hot_reload_app,
    }
}

fn main() {
    dbg!(TypeId::of::<Schedules>());
    let Symbols {
        create_app,
        get_hot_reload_detector,
        ..
    } = load_symbols();

    let mut app = create_app();

    loop {
        let reload_detected = get_hot_reload_detector(&app);
        reload_detected.store(false, Ordering::SeqCst);

        std::thread::spawn(move || {
            let modified = File::open(LIBRARY_PATH)
                .unwrap()
                .metadata()
                .unwrap()
                .modified()
                .unwrap();

            loop {
                let Ok(new_modified) = File::open(LIBRARY_PATH)
                    .and_then(|file| file.metadata())
                    .and_then(|metadata| metadata.modified())
                else {
                    std::thread::yield_now();
                    continue;
                };

                if new_modified > modified {
                    dbg!("Detected, reloading...");
                    reload_detected.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });

        let old_app = match dbg!(app.run()) {
            AppExitWithHotReload::HotReload(old_app) => old_app,
            _ => return,
        };

        let Symbols {
            create_app,
            hot_reload_app,
            ..
        } = load_symbols();

        app = hot_reload_app(old_app, create_app());
    }
}
