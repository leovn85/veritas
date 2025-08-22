#![recursion_limit = "256"]
#![allow(static_mut_refs)]
#![feature(windows_process_extensions_show_window)]
#[macro_use]
extern crate rust_i18n;

mod battle;
mod entry;
mod kreide;
mod logging;
mod models;
mod overlay;
mod prelude;
mod server;
mod subscribers;
mod ui;
mod updater;

use phf::phf_map;
use std::sync::LazyLock;
use tokio::runtime::Runtime;
use widestring::u16str;
use windows::{Win32::System::LibraryLoader::GetModuleHandleW, core::PCWSTR};

fn get_module_handle(name: &widestring::U16Str) -> usize {
    unsafe {
        GetModuleHandleW(PCWSTR(name.as_ptr()))
            .map(|v| v.0 as usize)
            .unwrap_or_else(|e| {
                log::error!("{e}");
                panic!("{e}");
            })
    }
}

pub static GAMEASSEMBLY_HANDLE: LazyLock<usize> =
    LazyLock::new(|| get_module_handle(u16str!("GameAssembly")));

pub static UNITYPLAYER_HANDLE: LazyLock<usize> =
    LazyLock::new(|| get_module_handle(u16str!("UnityPlayer")));

pub static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Runtime::new().unwrap_or_else(|e| {
        log::error!("{e}");
        panic!("{e}");
    })
});

pub const CHANGELOG: &str = include_str!("../CHANGELOG.MD");

static LOCALES: phf::Map<&'static str, &'static str> = phf_map! {
    "en" => "English",
    "fr" => "Français",
    "es" => "Español",
    "de" => "Deutsch",
    "it" => "Italiano",
    "ja" => "日本語",
    "nl" => "Nederlands",
    "pl" => "Polski",
    "pt" => "Português",
    "ru" => "Русский",
    "vi" => "Tiếng Việt",
    "zh" => "中文",
    "ar" => "العربية",
};

rust_i18n::i18n!();
