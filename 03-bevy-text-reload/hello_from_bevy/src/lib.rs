use std::{
    any::TypeId,
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
    println!("Hello from PSYCH!!");
}

#[unsafe(no_mangle)]
pub fn create_app() -> App {
    dbg!(TypeId::of::<Schedules>());

    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs(1))))
        .add_systems(Update, bevy_says_hi);

    // HOT RELOAD LOGIC
    app.insert_resource(HotReloadDetector(Arc::new(AtomicBool::new(false))))
        .add_systems(Last, detect_hot_reload);

    app
}

#[derive(Resource)]
struct HotReloadDetector(Arc<AtomicBool>);

fn detect_hot_reload(rx: Res<HotReloadDetector>, mut events: EventWriter<AppExit>) {
    if rx.into_inner().0.load(Ordering::Relaxed) {
        events.send(AppExit::HotReload);
    }
}

#[unsafe(no_mangle)]
pub fn get_hot_reload_detector(app: &App) -> Arc<AtomicBool> {
    Arc::clone(&app.world().resource::<HotReloadDetector>().0)
}

#[unsafe(no_mangle)]
pub fn hot_reload_app(mut old_app: App, mut new_app: App) -> App {
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

    old_app
}
