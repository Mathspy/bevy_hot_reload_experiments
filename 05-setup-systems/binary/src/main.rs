use std::{ffi::c_int, fs::File, process::ExitCode, sync::atomic::AtomicBool};

use libloading::Library;

const LIBRARY_PATH: &str = "../hello_from_bevy/target/release/libhello_from_bevy.dylib";

// These are just for our own sanity and for slight safety but they are just as good as a unit
struct App;
struct AppExitWithHotReload;

fn load_symbols() -> &'static mut Library {
    let lib = unsafe { Library::new(LIBRARY_PATH) }.unwrap();
    // For now we will leak the library because we don't want to close it on drop.
    Box::leak(Box::new(lib))
}

struct AppCreator(extern "C" fn() -> *mut App);

impl AppCreator {
    fn new(lib: &mut Library) -> Self {
        let create_app = unsafe { lib.get::<extern "C" fn() -> *mut App>(b"create_app") }.unwrap();

        AppCreator(*create_app)
    }
}

struct StaticSymbols {
    get_hot_reload_detector: unsafe extern "C" fn(app: *const App) -> *const AtomicBool,
    set_hot_reload_detector: unsafe extern "C" fn(bool: *const AtomicBool),
    run_app: unsafe extern "C" fn(*mut App) -> *mut AppExitWithHotReload,
    exit_code: unsafe extern "C" fn(exit: *mut AppExitWithHotReload) -> c_int,
    exit_into_app: unsafe extern "C" fn(exit: *mut AppExitWithHotReload) -> *mut App,
    hot_reload_reconciliation:
        unsafe extern "C" fn(old_app: *mut App, new_app: *mut App) -> *mut App,
}

impl StaticSymbols {
    fn new(lib: &mut Library) -> Self {
        let get_hot_reload_detector = unsafe {
            lib.get::<unsafe extern "C" fn(app: *const App) -> *const AtomicBool>(
                b"get_hot_reload_detector",
            )
        }
        .unwrap();
        let set_hot_reload_detector = unsafe {
            lib.get::<unsafe extern "C" fn(bool: *const AtomicBool)>(b"set_hot_reload_detector")
        }
        .unwrap();
        let run_app = unsafe {
            lib.get::<unsafe extern "C" fn(*mut App) -> *mut AppExitWithHotReload>(b"run_app")
        }
        .unwrap();
        let exit_code = unsafe {
            lib.get::<unsafe extern "C" fn(exit: *mut AppExitWithHotReload) -> c_int>(b"exit_code")
        }
        .unwrap();
        let exit_into_app = unsafe {
            lib.get::<unsafe extern "C" fn(exit: *mut AppExitWithHotReload) -> *mut App>(
                b"exit_into_app",
            )
        }
        .unwrap();
        let hot_reload_reconciliation = unsafe {
            lib.get::<unsafe extern "C" fn(old_app: *mut App, new_app: *mut App) -> *mut App>(
                b"hot_reload_reconciliation",
            )
        }
        .unwrap();

        StaticSymbols {
            get_hot_reload_detector: *get_hot_reload_detector,
            set_hot_reload_detector: *set_hot_reload_detector,
            run_app: *run_app,
            exit_code: *exit_code,
            exit_into_app: *exit_into_app,
            hot_reload_reconciliation: *hot_reload_reconciliation,
        }
    }
}

fn main() -> ExitCode {
    let lib = load_symbols();
    let app_creator = AppCreator::new(lib);
    let StaticSymbols {
        get_hot_reload_detector,
        set_hot_reload_detector,
        run_app,
        exit_code,
        exit_into_app,
        hot_reload_reconciliation,
    } = StaticSymbols::new(lib);

    let mut app = app_creator.0();

    loop {
        let reload_detected = PromiseItsIsSend(unsafe { get_hot_reload_detector(app) });

        struct PromiseItsIsSend(*const AtomicBool);
        impl PromiseItsIsSend {
            fn into_inner(self) -> *const AtomicBool {
                self.0
            }
        }
        unsafe impl Send for PromiseItsIsSend {}

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
                    unsafe { set_hot_reload_detector(reload_detected.into_inner()) };
                    break;
                }
            }
        });

        let output = unsafe { run_app(app) };
        let old_app = match unsafe { exit_code(output) } {
            0 => return ExitCode::SUCCESS,
            -1 => unsafe { exit_into_app(output) },
            int if int.is_positive() => return ExitCode::from(int as u8),
            _ => panic!("Unexpected negative exit_code"),
        };

        let lib = load_symbols();
        let app_creator = AppCreator::new(lib);

        app = unsafe { hot_reload_reconciliation(old_app, app_creator.0()) };
    }
}
