use crate::{kreide, logging, overlay, server, subscribers};
use ctor::ctor;
use egui_notify::Toast;
use std::{
    thread::{self},
    time::Duration,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

#[ctor]
fn entry() {
    thread::spawn(|| init());
}

fn init() {
    let mut toasts = Vec::<Toast>::new();
    let plugin_name = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    match setup_subscribers() {
        Ok(_) => {
            let msg = format!("Finished setting up {plugin_name}");
            log::info!("{}", msg);
            toasts.push(Toast::success(msg));
        }
        Err(e) => {
            let err = format!("{plugin_name} has been disabled: {e}");
            log::error!("{}", err);
            let mut toast = Toast::error(err);
            toast.duration(None);
            toasts.push(toast);
        }
    };

    match overlay::initialize(toasts) {
        Ok(_) => log::info!("Finished setting up overlay"),
        Err(e) => log::error!("Failed to initialize overlay: {}", e),
    }

    thread::spawn(|| {
        server::start_server();
    });
}

fn setup_subscribers() -> anyhow::Result<()> {
    unsafe {
        let plugin_name = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

        logging::MultiLogger::init()?;
        #[cfg(debug_assertions)]
        windows::Win32::System::Console::AllocConsole()?;

        log::info!("{plugin_name}");
        log::info!("Setting up...");

        while GetModuleHandleW(windows::core::w!("GameAssembly")).is_err()
            || GetModuleHandleW(windows::core::w!("UnityPlayer")).is_err()
        {
            thread::sleep(Duration::from_millis(10));
        }

        kreide::il2cpp::init()?;
        subscribers::battle::subscribe()?;
        subscribers::enable_subscribers!()?;
        Ok(())
    }
}
