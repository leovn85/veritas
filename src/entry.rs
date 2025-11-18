use crate::{kreide, logging, overlay, server, subscribers};
use ctor::ctor;
use egui_notify::Toast;
use std::{
    thread::{self},
    time::Duration,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug)]
pub enum InitErrorInfo {
    ObfuscationMismatch { class_name: Option<String>, message: String },
    Other { message: String },
}

pub static INIT_ERROR: LazyLock<Mutex<Option<InitErrorInfo>>> = LazyLock::new(|| Mutex::new(None));

pub fn take_init_error() -> Option<InitErrorInfo> {
    INIT_ERROR.lock().unwrap().take()
}

fn store_init_error(info: InitErrorInfo) {
    *INIT_ERROR.lock().unwrap() = Some(info);
}

#[ctor]
#[cfg(not(test))]
fn entry() {
    thread::spawn(|| init());
}

fn init() {
    logging::MultiLogger::init().unwrap();
    #[cfg(debug_assertions)]
    unsafe {
        windows::Win32::System::Console::AllocConsole().unwrap();
    }

    let mut toasts = Vec::<Toast>::new();
    let plugin_name = format!("{} ({})", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    log::info!("{}", plugin_name);
    match setup_subscribers() {
        Ok(_) => {
            let msg = format!("Core initialized successfully");
            log::info!("{}", msg);
            toasts.push(Toast::success(msg));
        }
        Err(e) => {
            let err = format!("Core failed to initialize and has been disabled: {e}");
            log::error!("{}", err);
            if let Some(info) = classify_init_error(&e) {
                store_init_error(info);
            }
            let mut toast = Toast::error(err);
            toast.duration(None);
            toasts.push(toast);
        }
    };

    thread::spawn(|| server::start_server());

    match overlay::initialize(toasts) {
        Ok(_) => log::info!("Overlay initialized successfully"),
        Err(e) => log::error!("Overlay failed to initialize: {}", e),
    }
}

fn setup_subscribers() -> anyhow::Result<()> {
    unsafe {
        log::info!("Setting up...");

        while GetModuleHandleW(windows::core::w!("GameAssembly")).is_err()
            || GetModuleHandleW(windows::core::w!("UnityPlayer")).is_err()
        {
            thread::sleep(Duration::from_secs(3));
        }

        kreide::il2cpp::init()?;
        subscribers::battle::subscribe()?;
        subscribers::enable_subscribers!()?;
        Ok(())
    }
}

fn classify_init_error(error: &anyhow::Error) -> Option<InitErrorInfo> {
    let message = error.to_string();
    if let Some(class_name) = extract_missing_class(&message) {
        Some(InitErrorInfo::ObfuscationMismatch {
            class_name: Some(class_name),
            message,
        })
    } else {
        Some(InitErrorInfo::Other { message })
    }
}

fn extract_missing_class(message: &str) -> Option<String> {
    let needle = "no such class";
    if !message.contains(needle) {
        return None;
    }

    let after = message.split(needle).nth(1)?;
    let class_name = after
        .trim()
        .split(|c: char| c.is_whitespace() || c == ':' || c == ')')
        .next()
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_matches('"').to_string())?;
    Some(class_name)
}
