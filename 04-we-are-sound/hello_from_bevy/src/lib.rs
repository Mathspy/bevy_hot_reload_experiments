use std::{
    any::TypeId,
    ffi::c_int,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bevy::{
    MinimalPlugins,
    app::{App, AppExit, AppExitWithHotReload, Last, PluginGroup, ScheduleRunnerPlugin, Update},
    ecs::{
        event::{EventWriter, Events},
        resource::Resource,
        schedule::Schedules,
        system::Res,
    },
};

fn bevy_says_hi() {
    println!("Hello from Bevy!");
}

/// True signature is () -> Box<App>
#[unsafe(no_mangle)]
pub extern "C" fn create_app() -> *mut App {
    dbg!(TypeId::of::<Schedules>());

    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs(1))))
        .add_systems(Update, bevy_says_hi);

    // HOT RELOAD LOGIC
    app.insert_resource(HotReloadDetector(Arc::new(AtomicBool::new(false))))
        .add_systems(Last, detect_hot_reload);

    Box::into_raw(Box::new(app))
}

#[derive(Resource)]
struct HotReloadDetector(Arc<AtomicBool>);

fn detect_hot_reload(rx: Res<HotReloadDetector>, mut events: EventWriter<AppExit>) {
    if rx.into_inner().0.load(Ordering::Relaxed) {
        events.send(AppExit::HotReload);
    }
}

/// # Safety
///
/// True signature is &App -> Arc<AtomicBool>
///
/// - Must be called with a valid opaque pointer to [`App`]
/// - [`App`] must not be modified until this function exits
#[unsafe(no_mangle)]
pub unsafe extern "C" fn get_hot_reload_detector(app: *const App) -> *const AtomicBool {
    let reload_detector = Arc::clone(&unsafe { &*app }.world().resource::<HotReloadDetector>().0);
    reload_detector.store(false, Ordering::SeqCst);
    Arc::into_raw(reload_detector)
}

/// # Safety
///
/// True signature is Arc<AtomicBool> -> ()
///
/// - Must be called with a valid Arc pointer to [`AtomicBool`]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn set_hot_reload_detector(bool: *const AtomicBool) {
    let reload_detector = unsafe { Arc::from_raw(bool) };
    reload_detector.store(true, Ordering::Relaxed);
}

/// # Safety
///
/// True signature is Box<App> -> Box<AppExitWithHotReload>
///
/// - Must be called with a valid opaque pointer to [`App`].
/// - This function consumes [`App`] so it must not be used in any way afterwards
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run_app(app: *mut App) -> *mut AppExitWithHotReload {
    let mut app = unsafe { Box::from_raw(app) };

    Box::into_raw(Box::new(app.run()))
}

/// # Safety
///
/// True signature is &AppExitWithHotReload -> i32
///
/// - Must be called with a valid opaque pointer to [`AppExitWithHotReload`].
/// - [`AppExitWithHotReload`] must not be modified until this function exits
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exit_code(exit: *mut AppExitWithHotReload) -> c_int {
    match unsafe { &*exit } {
        AppExitWithHotReload::Success => 0,
        AppExitWithHotReload::Error(non_zero) => non_zero.get().into(),
        AppExitWithHotReload::HotReload(_) => -1,
    }
}

/// This function will panic if [`AppExitWithHotReload`] is not HotReload(_)
///
/// # Safety
///
/// True signature is Box<AppExitWithHotReload> -> Box<App>
///
/// - Must be called with a valid opaque pointer to [`AppExitWithHotReload`].
/// - This function consumes [`AppExitWithHotReload`] so it must not be used in any way afterwards
#[unsafe(no_mangle)]
pub unsafe extern "C" fn exit_into_app(exit: *mut AppExitWithHotReload) -> *mut App {
    match *unsafe { Box::from_raw(exit) } {
        AppExitWithHotReload::HotReload(app) => Box::into_raw(Box::new(app)),
        _ => panic!("exit_into_app called with a non-hotreload AppExitWithHotReload"),
    }
}

/// # Safety
///
/// True signature is (Box<App>, Box<App>) -> Box<App>
///
/// - Must be called with two valid opaque pointers to two distinct to [`App`]s.
/// - This function consumes both [`App`]s so they must not be used in any way afterwards
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hot_reload_reconciliation(
    old_app: *mut App,
    new_app: *mut App,
) -> *mut App {
    let mut old_app = unsafe { Box::from_raw(old_app) };
    let mut new_app = unsafe { Box::from_raw(new_app) };

    old_app
        .world_mut()
        .resource_mut::<Events<AppExit>>()
        .into_inner()
        .clear();

    let updated_schedules = new_app.world_mut().remove_resource::<Schedules>().unwrap();
    old_app.insert_resource(updated_schedules);

    old_app.set_runner(std::mem::replace(
        new_app.get_runner_mut(),
        Box::new(|_: App| -> AppExitWithHotReload {
            unreachable!();
        }),
    ));

    Box::into_raw(old_app)
}
