use std::sync::{Mutex, RwLock, atomic::{AtomicU32, Ordering}};

use edio11::{PostRenderContext, WindowMessage};
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_WRITE, MapViewOfFile, PAGE_READWRITE,
};
use windows::core::{Interface, PCWSTR};
use widestring::U16CString;

pub const PLUGIN_API_VERSION: u32 = 3;
pub const PLUGIN_BUS_MAGIC: u32 = 0x6D656F77;
pub const VERITAS_BUS_MAP_NAME: &str = "Local\\VeritasPluginBus_v3";

#[repr(C)]
pub struct BattleCallbacks {
    pub on_battle_begin: Option<unsafe extern "C" fn(game_mode_ptr: usize)>,
    pub on_battle_end: Option<unsafe extern "C" fn()>,
    pub on_set_lineup: Option<unsafe extern "C" fn(instance_ptr: usize, lineup_data_ptr: usize)>,
    pub on_turn_begin: Option<unsafe extern "C" fn(game_mode_ptr: usize)>,
    pub on_init_enemy: Option<unsafe extern "C" fn(component_ptr: usize)>,
    pub on_use_skill: Option<unsafe extern "C" fn(component_ptr: usize, skill_index: i32, extra: i32)>,
    pub on_battle_end_with_result:
        Option<unsafe extern "C" fn(total_damage: f64, action_value: f64, turn_count: u32, cycle: u32)>,
    pub on_damage: Option<unsafe extern "C" fn(attacker_uid: u32, damage: f64)>,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct PluginWindowMessage {
    pub hwnd: usize,
    pub msg: u32,
    pub _pad: u32,
    pub wparam: usize,
    pub lparam: isize,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct PluginSurfaceInputResult {
    pub capture_pointer_input: bool,
    pub capture_keyboard_input: bool,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct PluginSurfaceRenderContext {
    pub hwnd: usize,
    pub device_context: usize,
    pub render_target: usize,
    pub pixels_per_point: f32,
    pub _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PluginDescriptor {
    pub api_version: u32,
    pub name: [u8; 64],
    pub version: [u8; 32],
    pub battle_callbacks: *const BattleCallbacks,
    pub handle_action: Option<unsafe extern "C" fn(widget_id: u32, value: f64)>,
    pub handle_text: Option<unsafe extern "C" fn(widget_id: u32, text_ptr: *const u8, text_len: u32)>,
    pub process_window_message:
        Option<unsafe extern "C" fn(message: *const PluginWindowMessage, out: *mut PluginSurfaceInputResult)>,
    pub render_surface: Option<unsafe extern "C" fn(ctx: *const PluginSurfaceRenderContext)>,
}

impl PluginDescriptor {
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&byte| byte == 0).unwrap_or(self.name.len());
        std::str::from_utf8(&self.name[..end]).unwrap_or("<invalid>")
    }

    pub fn version_str(&self) -> &str {
        let end = self.version.iter().position(|&byte| byte == 0).unwrap_or(self.version.len());
        std::str::from_utf8(&self.version[..end]).unwrap_or("<invalid>")
    }
}

unsafe impl Send for PluginDescriptor {}
unsafe impl Sync for PluginDescriptor {}

#[repr(C)]
pub struct PluginBus {
    pub magic: u32,
    pub api_version: u32,
    pub register: unsafe extern "C" fn(desc: *const PluginDescriptor) -> bool,
    pub apply_theme: unsafe extern "C" fn(ctx: *mut ()),
    pub get_window_opacity: unsafe extern "C" fn() -> f32,
    pub log_record: unsafe extern "C" fn(
        level: u8,
        target_ptr: *const u8,
        target_len: usize,
        msg_ptr: *const u8,
        msg_len: usize,
    ),
    pub hook: unsafe extern "C" fn(target_va: usize, detour_fn: usize, trampoline_out: *mut usize) -> bool,
    pub unhook: unsafe extern "C" fn(target_va: usize) -> bool,
}

struct PluginDetour {
    target_va: usize,
    detour: retour::RawDetour,
}

unsafe impl Send for PluginDetour {}

const MAX_PLUGINS: usize = 16;

static PLUGINS: Mutex<Vec<PluginDescriptor>> = Mutex::new(Vec::new());
static PLUGIN_HOOKS: Mutex<Vec<PluginDetour>> = Mutex::new(Vec::new());
static CURRENT_STYLE: RwLock<Option<std::sync::Arc<egui::Style>>> = RwLock::new(None);
static CURRENT_THEME: RwLock<Option<egui::Theme>> = RwLock::new(None);
static CURRENT_WINDOW_OPACITY_BITS: AtomicU32 = AtomicU32::new(f32::to_bits(0.30));

#[unsafe(no_mangle)]
pub unsafe extern "C" fn veritas_register_plugin(desc: *const PluginDescriptor) -> bool {
    if desc.is_null() {
        return false;
    }

    let desc_ref = unsafe { &*desc };
    if desc_ref.api_version != PLUGIN_API_VERSION {
        log::error!(
            "[veritas::plugin] rejected: api_version={} expected={}",
            desc_ref.api_version,
            PLUGIN_API_VERSION
        );
        return false;
    }

    let mut plugins = PLUGINS.lock().unwrap();
    if plugins.len() >= MAX_PLUGINS {
        log::error!(
            "[veritas::plugin] plugin list full, rejecting '{}'",
            desc_ref.name_str()
        );
        return false;
    }

    plugins.push(unsafe { std::ptr::read(desc) });
    log::info!(
        "[veritas::plugin] registered '{}' v{}",
        desc_ref.name_str(),
        desc_ref.version_str()
    );
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn veritas_apply_theme(ctx_ptr: *mut ()) {
    if ctx_ptr.is_null() {
        return;
    }

    apply_theme_to_extern_ctx(unsafe { &*(ctx_ptr as *const egui::Context) });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn veritas_get_window_opacity() -> f32 {
    f32::from_bits(CURRENT_WINDOW_OPACITY_BITS.load(Ordering::Relaxed))
}

unsafe extern "C" fn log_record_trampoline(
    level: u8,
    target_ptr: *const u8,
    target_len: usize,
    msg_ptr: *const u8,
    msg_len: usize,
) {
    crate::veritas_log_record(level, target_ptr, target_len, msg_ptr, msg_len);
}

unsafe extern "C" fn veritas_hook_method(
    target_va: usize,
    detour_fn: usize,
    trampoline_out: *mut usize,
) -> bool {
    match unsafe { retour::RawDetour::new(target_va as *const (), detour_fn as *const ()) } {
        Ok(detour) => {
            if unsafe { detour.enable() }.is_err() {
                log::error!("[veritas::plugin] hook enable failed at {target_va:#x}");
                return false;
            }

            if !trampoline_out.is_null() {
                unsafe { *trampoline_out = detour.trampoline() as *const () as usize };
            }

            log::info!("[veritas::plugin] hook installed at {target_va:#x}");
            PLUGIN_HOOKS.lock().unwrap().push(PluginDetour { target_va, detour });
            true
        }
        Err(error) => {
            log::error!("[veritas::plugin] hook failed at {target_va:#x}: {error}");
            false
        }
    }
}

unsafe extern "C" fn veritas_unhook_method(target_va: usize) -> bool {
    let mut hooks = PLUGIN_HOOKS.lock().unwrap();
    if let Some(position) = hooks.iter().position(|hook| hook.target_va == target_va) {
        let hook = hooks.remove(position);
        let _ = unsafe { hook.detour.disable() };
        log::info!("[veritas::plugin] hook removed at {target_va:#x}");
        true
    } else {
        false
    }
}

static BUS: std::sync::LazyLock<PluginBus> = std::sync::LazyLock::new(|| PluginBus {
    magic: PLUGIN_BUS_MAGIC,
    api_version: PLUGIN_API_VERSION,
    register: veritas_register_plugin,
    apply_theme: veritas_apply_theme,
    get_window_opacity: veritas_get_window_opacity,
    log_record: log_record_trampoline,
    hook: veritas_hook_method,
    unhook: veritas_unhook_method,
});

pub fn publish_bus() {
    let name = match U16CString::from_str(VERITAS_BUS_MAP_NAME) {
        Ok(name) => name,
        Err(error) => {
            log::error!("[veritas::plugin] publish_bus: bad mapping name: {error}");
            return;
        }
    };
    unsafe {
        let mapping = CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            None,
            PAGE_READWRITE,
            0,
            std::mem::size_of::<usize>() as u32,
            PCWSTR(name.as_ptr()),
        );
        match mapping {
            Ok(handle) => {
                let view = MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, 0);
                if view.Value.is_null() {
                    log::error!("[veritas::plugin] MapViewOfFile failed");
                } else {
                    let bus_ptr = &*BUS as *const PluginBus as usize;
                    *(view.Value as *mut usize) = bus_ptr;
                    log::info!("[veritas::plugin] bus published at {bus_ptr:#x}");
                }
            }
            Err(error) => log::error!("[veritas::plugin] CreateFileMappingW failed: {error}"),
        }
    }
}

pub fn update_current_style(ctx: &egui::Context) {
    if let Ok(mut style) = CURRENT_STYLE.write() {
        *style = Some(ctx.style());
    }
    if let Ok(mut theme) = CURRENT_THEME.write() {
        *theme = Some(ctx.theme());
    }
}

pub fn update_window_opacity(opacity: f32) {
    CURRENT_WINDOW_OPACITY_BITS.store(opacity.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

pub fn apply_theme_to_extern_ctx(ctx: &egui::Context) {
    if let Ok(theme) = CURRENT_THEME.read() {
        if let Some(theme) = *theme {
            ctx.set_theme(theme);
        }
    }

    if let Ok(style) = CURRENT_STYLE.read() {
        if let Some(style) = style.as_ref() {
            ctx.set_style((**style).clone());
        }
    }
}

pub fn plugin_count() -> usize {
    PLUGINS.lock().map(|plugins| plugins.len()).unwrap_or(0)
}

fn format_panic_box(error: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = error.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = error.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_owned()
    }
}

macro_rules! for_each_battle_cb {
    ($field:ident, $call:expr) => {{
        let callbacks: Vec<_> = {
            let plugins = PLUGINS.lock().unwrap();
            plugins
                .iter()
                .filter(|plugin| !plugin.battle_callbacks.is_null())
                .filter_map(|plugin| unsafe { (*plugin.battle_callbacks).$field })
                .collect()
        };

        for callback in callbacks {
            match microseh::try_seh(|| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $call(callback)))
            }) {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let message = format!(
                        "[veritas::plugin] panic in battle dispatch {}: {}",
                        stringify!($field),
                        format_panic_box(&error)
                    );
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        log::error!("{}", message)
                    }));
                }
                Err(error) => {
                    let message = format!(
                        "[veritas::plugin] SEH in battle dispatch {}: {:?}",
                        stringify!($field),
                        error
                    );
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        log::error!("{}", message)
                    }));
                }
            }
        }
    }};
}

pub fn dispatch_battle_begin(game_mode_ptr: usize) {
    for_each_battle_cb!(on_battle_begin, |callback: unsafe extern "C" fn(usize)| unsafe {
        callback(game_mode_ptr)
    });
}

pub fn dispatch_battle_end() {
    for_each_battle_cb!(on_battle_end, |callback: unsafe extern "C" fn()| unsafe {
        callback()
    });
}

pub fn dispatch_battle_end_with_result(total_damage: f64, action_value: f64, turn_count: u32, cycle: u32) {
    for_each_battle_cb!(
        on_battle_end_with_result,
        |callback: unsafe extern "C" fn(f64, f64, u32, u32)| unsafe {
            callback(total_damage, action_value, turn_count, cycle)
        }
    );
}

pub fn dispatch_on_damage(attacker_uid: u32, damage: f64) {
    for_each_battle_cb!(on_damage, |callback: unsafe extern "C" fn(u32, f64)| unsafe {
        callback(attacker_uid, damage)
    });
}

pub fn dispatch_on_set_lineup(instance_ptr: usize, lineup_data_ptr: usize) {
    for_each_battle_cb!(on_set_lineup, |callback: unsafe extern "C" fn(usize, usize)| unsafe {
        callback(instance_ptr, lineup_data_ptr)
    });
}

pub fn dispatch_on_turn_begin(game_mode_ptr: usize) {
    for_each_battle_cb!(on_turn_begin, |callback: unsafe extern "C" fn(usize)| unsafe {
        callback(game_mode_ptr)
    });
}

pub fn dispatch_on_init_enemy(component_ptr: usize) {
    for_each_battle_cb!(on_init_enemy, |callback: unsafe extern "C" fn(usize)| unsafe {
        callback(component_ptr)
    });
}

pub fn dispatch_on_use_skill(component_ptr: usize, skill_index: i32, extra: i32) {
    for_each_battle_cb!(
        on_use_skill,
        |callback: unsafe extern "C" fn(usize, i32, i32)| unsafe {
            callback(component_ptr, skill_index, extra)
        }
    );
}

pub fn process_plugin_surface_message(message: &WindowMessage) -> PluginSurfaceInputResult {
    let callbacks: Vec<_> = {
        let plugins = PLUGINS.lock().unwrap();
        plugins
            .iter()
            .filter_map(|plugin| plugin.process_window_message)
            .collect()
    };

    let plugin_message = PluginWindowMessage {
        hwnd: message.hwnd.0 as usize,
        msg: message.msg,
        _pad: 0,
        wparam: message.wparam.0,
        lparam: message.lparam.0,
    };

    let mut combined = PluginSurfaceInputResult::default();
    for callback in callbacks {
        let mut out = PluginSurfaceInputResult::default();
        match microseh::try_seh(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                callback(&plugin_message, &mut out);
            }))
        }) {
            Ok(Ok(())) => {
                combined.capture_pointer_input |= out.capture_pointer_input;
                combined.capture_keyboard_input |= out.capture_keyboard_input;
            }
            Ok(Err(error)) => {
                log::error!(
                    "[veritas::plugin] panic in process_window_message: {}",
                    format_panic_box(&error)
                );
            }
            Err(error) => {
                log::error!("[veritas::plugin] SEH in process_window_message: {:?}", error);
            }
        }
    }

    combined
}

pub fn render_all_plugin_surfaces(ctx: &PostRenderContext<'_>) {
    let callbacks: Vec<_> = {
        let plugins = PLUGINS.lock().unwrap();
        plugins.iter().filter_map(|plugin| plugin.render_surface).collect()
    };

    let render_context = PluginSurfaceRenderContext {
        hwnd: ctx.hwnd.0 as usize,
        device_context: ctx.device_context.as_raw() as usize,
        render_target: ctx.render_target.as_raw() as usize,
        pixels_per_point: ctx.pixels_per_point,
        _pad: 0,
    };

    for callback in callbacks {
        match microseh::try_seh(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
                callback(&render_context);
            }))
        }) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                log::error!(
                    "[veritas::plugin] panic in render_surface: {}",
                    format_panic_box(&error)
                );
            }
            Err(error) => {
                log::error!("[veritas::plugin] SEH in render_surface: {:?}", error);
            }
        }
    }
}
