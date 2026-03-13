use std::sync::{Mutex, RwLock};
use windows::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows::Win32::System::Memory::{
    CreateFileMappingW, FILE_MAP_WRITE, MapViewOfFile, PAGE_READWRITE,
};
use windows::core::PCWSTR;
use widestring::U16CString;

pub const PLUGIN_API_VERSION: u32 = 1;

pub const PLUGIN_BUS_MAGIC: u32 = 0x6D656F77;

pub const VERITAS_BUS_MAP_NAME: &str = "Local\\VeritasPluginBus_v1";

// battle event callbacks
#[repr(C)]
pub struct BattleCallbacks {
    pub on_battle_begin: Option<unsafe extern "C" fn(game_mode_ptr: usize)>,
    pub on_battle_end: Option<unsafe extern "C" fn()>,
    pub on_set_lineup: Option<unsafe extern "C" fn(instance_ptr: usize, lineup_data_ptr: usize)>,
    pub on_turn_begin: Option<unsafe extern "C" fn(game_mode_ptr: usize)>,
    pub on_init_enemy: Option<unsafe extern "C" fn(component_ptr: usize)>,
    pub on_use_skill: Option<unsafe extern "C" fn(component_ptr: usize, skill_index: i32, extra: i32)>,
    pub on_battle_end_with_result: Option<unsafe extern "C" fn(total_damage: f64, action_value: f64, turn_count: u32, cycle: u32)>,
    pub on_damage: Option<unsafe extern "C" fn(attacker_uid: u32, damage: f64)>,
}

pub const WIDGET_LABEL:            u8 = 0;
pub const WIDGET_CHECKBOX:         u8 = 1;
pub const WIDGET_SLIDER:           u8 = 2;
pub const WIDGET_BUTTON:           u8 = 3;
pub const WIDGET_SEPARATOR:        u8 = 4;
pub const WIDGET_PROGRESS:         u8 = 5;
pub const WIDGET_TEXT_EDIT:        u8 = 6;
pub const WIDGET_HEADING:          u8 = 7;
pub const WIDGET_DRAG_VALUE:       u8 = 8;
pub const WIDGET_COLOR_LABEL:      u8 = 9;
pub const WIDGET_HORIZONTAL_BEGIN: u8 = 10;
pub const WIDGET_HORIZONTAL_END:   u8 = 11;
// table: .label=tab-separated column headers, .value=heat_min, .min_value=heat_max, .id=heat_column(1-based, 0=none)
pub const WIDGET_TABLE_BEGIN: u8 = 12;
// table row: .label=tab-separated cell values (\n within a cell = two-line cell), .value=heat_value
pub const WIDGET_TABLE_ROW:   u8 = 13;

pub const WIDGET_FLAG_ENABLED:  u8 = 1 << 0;
pub const WIDGET_FLAG_MONOSPACE: u8 = 1 << 1;
pub const WIDGET_FLAG_SMALL:    u8 = 1 << 2;
pub const WIDGET_FLAG_WEAK:     u8 = 1 << 3;
pub const WIDGET_FLAG_STRONG:   u8 = 1 << 4;

pub const MAX_PLUGIN_WIDGETS: usize = 128;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PluginWidgetDesc {
    pub id:        u32,   // used by handle_action to identify the widget
    pub kind:      u8,
    pub flags:     u8,
    pub _pad:      [u8; 2],
    pub label:     [u8; 128],
    // checkbox: 0/1; slider/drag: current; progress: current; color_label: packed rgba u32; text_edit: desired_rows
    pub value:     f64,
    pub min_value: f64,   // slider: min; progress: total; drag: speed
    pub max_value: f64,   // slider/drag: max
    pub data_ptr:  usize, // text_edit: current text bytes
    pub data_len:  u32,
    pub _pad2:     u32,
}

#[repr(C)]
pub struct PluginPanelDesc {
    pub title:        [u8; 64],
    pub widget_count: u32,
    pub flags:        u32, // bit 0: panel starts visible
    pub widgets:      [PluginWidgetDesc; MAX_PLUGIN_WIDGETS],
}

impl PluginPanelDesc {
    pub fn title_str(&self) -> &str {
        let end = self.title.iter().position(|&b| b == 0).unwrap_or(self.title.len());
        std::str::from_utf8(&self.title[..end]).unwrap_or("<invalid>")
    }
}

impl PluginWidgetDesc {
    pub fn label_str(&self) -> &str {
        let end = self.label.iter().position(|&b| b == 0).unwrap_or(self.label.len());
        std::str::from_utf8(&self.label[..end]).unwrap_or("<invalid>")
    }
    pub fn is_enabled(&self) -> bool {
        self.flags & WIDGET_FLAG_ENABLED != 0
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PluginDescriptor {
    pub api_version: u32, // must equal PLUGIN_API_VERSION

    pub name:    [u8; 64],
    pub version: [u8; 32],

    pub render_menu_section: Option<unsafe extern "C" fn(ui: *mut ())>,
    pub wants_panel_open:    Option<unsafe extern "C" fn() -> bool>,
    pub render_window:       Option<unsafe extern "C" fn(ctx: *const ())>,

    // null when the plugin manages its own hooks; otherwise veritas calls these from its hooks
    pub battle_callbacks: *const BattleCallbacks,

    pub get_panel_desc: Option<unsafe extern "C" fn(out: *mut PluginPanelDesc)>,
    pub handle_action:  Option<unsafe extern "C" fn(widget_id: u32, value: f64)>,
    pub handle_text:    Option<unsafe extern "C" fn(widget_id: u32, text_ptr: *const u8, text_len: u32)>,
}

impl PluginDescriptor {
    pub fn name_str(&self) -> &str {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(self.name.len());
        std::str::from_utf8(&self.name[..end]).unwrap_or("<invalid>")
    }

    pub fn version_str(&self) -> &str {
        let end = self.version.iter().position(|&b| b == 0).unwrap_or(self.version.len());
        std::str::from_utf8(&self.version[..end]).unwrap_or("<invalid>")
    }
}

unsafe impl Send for PluginDescriptor {}
unsafe impl Sync for PluginDescriptor {}

#[repr(C)]
pub struct PluginBus {
    pub magic:       u32,
    pub api_version: u32,
    pub register:    unsafe extern "C" fn(desc: *const PluginDescriptor) -> bool,
    pub apply_theme: unsafe extern "C" fn(ctx: *mut ()),
    pub log_record:  unsafe extern "C" fn(
        level:      u8,
        target_ptr: *const u8, target_len: usize,
        msg_ptr:    *const u8, msg_len:    usize,
    ),
    /// install a detour at `target_va`. On success writes the trampoline address
    /// to `*trampoline_out` and returns `true`.
    pub hook:   unsafe extern "C" fn(target_va: usize, detour_fn: usize, trampoline_out: *mut usize) -> bool,
    /// remove a previously installed plugin detour; eturns `true` if found
    pub unhook: unsafe extern "C" fn(target_va: usize) -> bool,
}

const MAX_PLUGINS: usize = 16;

static PLUGINS: Mutex<Vec<PluginDescriptor>> = Mutex::new(Vec::new());

static CURRENT_STYLE: RwLock<Option<std::sync::Arc<egui::Style>>> = RwLock::new(None);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn veritas_register_plugin(desc: *const PluginDescriptor) -> bool {
    if desc.is_null() {
        return false;
    }
    let desc_ref = &*desc;
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
    plugins.push(std::ptr::read(desc));
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
    apply_theme_to_extern_ctx(&*(ctx_ptr as *const egui::Context));
}

unsafe extern "C" fn log_record_trampoline(
    level:      u8,
    target_ptr: *const u8, target_len: usize,
    msg_ptr:    *const u8, msg_len:    usize,
) {
    crate::veritas_log_record(level, target_ptr, target_len, msg_ptr, msg_len);
}

struct PluginDetour {
    target_va: usize,
    detour: retour::RawDetour,
}
unsafe impl Send for PluginDetour {}

static PLUGIN_HOOKS: Mutex<Vec<PluginDetour>> = Mutex::new(Vec::new());

unsafe extern "C" fn veritas_hook_method(
    target_va: usize,
    detour_fn: usize,
    trampoline_out: *mut usize,
) -> bool {
    match retour::RawDetour::new(target_va as *const (), detour_fn as *const ()) {
        Ok(d) => {
            if d.enable().is_err() {
                log::error!("[veritas::plugin] hook enable failed at {target_va:#x}");
                return false;
            }
            if !trampoline_out.is_null() {
                *trampoline_out = d.trampoline() as *const () as usize;
            }
            log::info!("[veritas::plugin] hook installed at {target_va:#x}");
            PLUGIN_HOOKS.lock().unwrap().push(PluginDetour { target_va, detour: d });
            true
        }
        Err(e) => {
            log::error!("[veritas::plugin] hook failed at {target_va:#x}: {e}");
            false
        }
    }
}

unsafe extern "C" fn veritas_unhook_method(target_va: usize) -> bool {
    let mut hooks = PLUGIN_HOOKS.lock().unwrap();
    if let Some(pos) = hooks.iter().position(|h| h.target_va == target_va) {
        let h = hooks.remove(pos);
        let _ = h.detour.disable();
        log::info!("[veritas::plugin] hook removed at {target_va:#x}");
        true
    } else {
        false
    }
}

static BUS: std::sync::LazyLock<PluginBus> = std::sync::LazyLock::new(|| PluginBus {
    magic:       PLUGIN_BUS_MAGIC,
    api_version: PLUGIN_API_VERSION,
    register:    veritas_register_plugin,
    apply_theme: veritas_apply_theme,
    log_record:  log_record_trampoline,
    hook:        veritas_hook_method,
    unhook:      veritas_unhook_method,
});

pub fn publish_bus() {
    let name = match U16CString::from_str(VERITAS_BUS_MAP_NAME) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[veritas::plugin] publish_bus: bad mapping name: {e}");
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
                std::mem::forget(handle);
            }
            Err(e) => log::error!("[veritas::plugin] CreateFileMappingW failed: {e}"),
        }
    }
}

pub fn update_current_style(ctx: &egui::Context) {
    if let Ok(mut w) = CURRENT_STYLE.write() {
        *w = Some(ctx.style());
    }
}

pub fn apply_theme_to_extern_ctx(ctx: &egui::Context) {
    if let Ok(r) = CURRENT_STYLE.read() {
        if let Some(style) = r.as_ref() {
            ctx.set_style((**style).clone());
        }
    }
}

pub fn render_all_menu_sections(ui: &mut egui::Ui) {
    let plugins = PLUGINS.lock().unwrap();
    let mut vis = PANEL_VISIBILITY.lock().unwrap();
    while vis.len() < plugins.len() {
        vis.push(true);
    }
    for (i, p) in plugins.iter().enumerate() {
        if p.get_panel_desc.is_some() {
            ui.label(
                egui::RichText::new(p.name_str())
                    .small()
                    .color(egui::Color32::from_rgb(100, 220, 100)),
            );
            ui.checkbox(&mut vis[i], format!("Show {} Panel", p.name_str()));
        }
    }
}

pub fn render_all_plugin_panels(ctx: &egui::Context) {
    let panel_fns: Vec<(
        usize,
        String,
        unsafe extern "C" fn(*mut PluginPanelDesc),
        Option<unsafe extern "C" fn(u32, f64)>,
        Option<unsafe extern "C" fn(u32, *const u8, u32)>,
    )> = {
        let plugins = PLUGINS.lock().unwrap();
        let mut vis = PANEL_VISIBILITY.lock().unwrap();
        while vis.len() < plugins.len() {
            vis.push(true);
        }
        plugins
            .iter()
            .enumerate()
            .filter(|(i, _)| vis.get(*i).copied().unwrap_or(true))
            .filter_map(|(i, p)| {
                p.get_panel_desc.map(|gp| (i, p.name_str().to_owned(), gp, p.handle_action, p.handle_text))
            })
            .collect()
    };

    for (i, name, get_desc, handle_action, handle_text) in panel_fns {
        let mut desc = unsafe { std::mem::zeroed::<PluginPanelDesc>() };
        let seh = microseh::try_seh(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                unsafe { get_desc(&mut desc) };
            }))
        });
        match seh {
            Err(e) => {
                log::error!("[veritas::plugin] SEH in get_panel_desc from '{}': {:?}", name, e);
                continue;
            }
            Ok(Err(e)) => {
                log::error!("[veritas::plugin] panic in get_panel_desc from '{}': {}", name, format_panic_box(&e));
                continue;
            }
            Ok(Ok(())) => {}
        }

        if desc.widget_count == 0 {
            continue;
        }

        render_generic_panel(ctx, &desc, handle_action, handle_text, i);
    }
}

fn render_generic_panel(
    ctx: &egui::Context,
    desc: &PluginPanelDesc,
    handle_action: Option<unsafe extern "C" fn(u32, f64)>,
    handle_text: Option<unsafe extern "C" fn(u32, *const u8, u32)>,
    plugin_idx: usize,
) {
    let title = desc.title_str();
    let count = (desc.widget_count as usize).min(MAX_PLUGIN_WIDGETS);
    egui::Window::new(title)
        .id(egui::Id::new(("__plugin_panel", plugin_idx)))
        .resizable(true)
        .default_width(480.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                let mut in_horizontal = false;
                let mut i = 0;
                while i < count {
                    let w = &desc.widgets[i];
                    match w.kind {
                        WIDGET_LABEL => {
                            let mut rt = egui::RichText::new(w.label_str());
                            if w.flags & WIDGET_FLAG_SMALL != 0  { rt = rt.small(); }
                            if w.flags & WIDGET_FLAG_WEAK != 0   { rt = rt.weak(); }
                            if w.flags & WIDGET_FLAG_STRONG != 0 { rt = rt.strong(); }
                            ui.label(rt);
                        }
                        WIDGET_CHECKBOX => {
                            let mut checked = w.value != 0.0;
                            if ui.add_enabled(w.is_enabled(), egui::Checkbox::new(&mut checked, w.label_str())).changed() {
                                if let Some(f) = handle_action {
                                    safe_action_call(f, w.id, if checked { 1.0 } else { 0.0 });
                                }
                            }
                        }
                        WIDGET_SLIDER => {
                            let mut val = w.value as f32;
                            let min = w.min_value as f32;
                            let max = w.max_value as f32;
                            if ui.add_enabled(
                                w.is_enabled(),
                                egui::Slider::new(&mut val, min..=max)
                                    .text(w.label_str())
                                    .logarithmic(max / min.max(0.001) > 100.0),
                            ).changed() {
                                if let Some(f) = handle_action {
                                    safe_action_call(f, w.id, val as f64);
                                }
                            }
                        }
                        WIDGET_BUTTON => {
                            let mut rt = egui::RichText::new(w.label_str());
                            if w.flags & WIDGET_FLAG_SMALL != 0  { rt = rt.small(); }
                            if w.flags & WIDGET_FLAG_STRONG != 0 { rt = rt.strong(); }
                            if ui.add_enabled(w.is_enabled(), egui::Button::new(rt)).clicked() {
                                if let Some(f) = handle_action {
                                    safe_action_call(f, w.id, 0.0);
                                }
                            }
                        }
                        WIDGET_SEPARATOR => {
                            ui.separator();
                        }
                        WIDGET_PROGRESS => {
                            let cur = w.value as f32;
                            let total = w.min_value as f32;
                            let pct = if total > 0.0 { cur / total } else { 0.0 };
                            ui.horizontal(|ui| {
                                ui.label(w.label_str());
                                let bar_color = egui::Color32::from_rgb(80, 180, 100);
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 10.0), egui::Sense::hover());
                                let painter = ui.painter();
                                painter.rect_filled(rect, 3.0, egui::Color32::from_gray(50));
                                let fill = egui::Rect::from_min_size(
                                    rect.min,
                                    egui::vec2(rect.width() * pct.clamp(0.0, 1.0), rect.height()),
                                );
                                painter.rect_filled(fill, 3.0, bar_color);
                                ui.label(format!("{}/{}", cur as u32, total as u32));
                            });
                        }
                        WIDGET_TEXT_EDIT => {
                            let current = if w.data_ptr != 0 && w.data_len > 0 {
                                let slice = unsafe {
                                    std::slice::from_raw_parts(w.data_ptr as *const u8, w.data_len as usize)
                                };
                                std::str::from_utf8(slice).unwrap_or("").to_owned()
                            } else {
                                String::new()
                            };
                            let mut text = current.clone();
                            let rows = (w.value as usize).max(3);
                            let monospace = w.flags & WIDGET_FLAG_MONOSPACE != 0;
                            let mut te = egui::TextEdit::multiline(&mut text)
                                .hint_text(w.label_str())
                                .desired_width(f32::INFINITY)
                                .desired_rows(rows);
                            if monospace { te = te.font(egui::TextStyle::Monospace); }
                            let resp = ui.add(te);
                            if resp.changed() && text != current {
                                if let Some(f) = handle_text {
                                    safe_text_call(f, w.id, &text);
                                }
                            }
                        }
                        WIDGET_HEADING => {
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(w.label_str()).strong().small());
                        }
                        WIDGET_DRAG_VALUE => {
                            let mut val = w.value;
                            let speed = w.min_value.max(1.0);
                            ui.horizontal(|ui| {
                                ui.label(w.label_str());
                                if ui.add_enabled(
                                    w.is_enabled(),
                                    egui::DragValue::new(&mut val).speed(speed),
                                ).changed() {
                                    if let Some(f) = handle_action {
                                        safe_action_call(f, w.id, val);
                                    }
                                }
                            });
                        }
                        WIDGET_COLOR_LABEL => {
                            let packed = w.value as u32;
                            let r = ((packed >> 24) & 0xFF) as u8;
                            let g = ((packed >> 16) & 0xFF) as u8;
                            let b = ((packed >> 8) & 0xFF) as u8;
                            let color = egui::Color32::from_rgb(r, g, b);
                            let mut rt = egui::RichText::new(w.label_str()).color(color);
                            if w.flags & WIDGET_FLAG_SMALL != 0  { rt = rt.small(); }
                            if w.flags & WIDGET_FLAG_STRONG != 0 { rt = rt.strong(); }
                            if w.flags & WIDGET_FLAG_MONOSPACE != 0 {
                                rt = rt.font(egui::FontId::monospace(11.0));
                            }
                            ui.label(rt);
                        }
                        WIDGET_HORIZONTAL_BEGIN => {
                            in_horizontal = true;
                            let start = i + 1;
                            let mut end = start;
                            while end < count && desc.widgets[end].kind != WIDGET_HORIZONTAL_END {
                                end += 1;
                            }
                            ui.horizontal(|ui| {
                                for hw in &desc.widgets[start..end] {
                                    render_inline_widget(ui, hw, handle_action, handle_text);
                                }
                            });
                            i = if end < count { end + 1 } else { end };
                            in_horizontal = false;
                            continue;
                        }
                        WIDGET_HORIZONTAL_END => {}
                        WIDGET_TABLE_BEGIN => {
                            let heat_min = w.value;
                            let heat_max = w.min_value;
                            let heat_col = w.id as usize; // 1-based column index, 0 = no heat

                            let headers: Vec<&str> = w.label_str().split('\t').collect();
                            let num_cols = headers.len().max(1);

                            let rows_start = i + 1;
                            let mut rows_end = rows_start;
                            while rows_end < count && desc.widgets[rows_end].kind == WIDGET_TABLE_ROW {
                                rows_end += 1;
                            }

                            let stripe_dark  = egui::Color32::from_rgba_premultiplied(15, 20, 35, 120);
                            let stripe_light = egui::Color32::TRANSPARENT;
                            let header_bg    = egui::Color32::from_rgba_premultiplied(25, 30, 50, 220);
                            let divider_col  = egui::Color32::from_gray(60);
                            let cell_pad     = egui::vec2(8.0, 5.0);

                            let fixed_w = if w.max_value > 0.0 { w.max_value as f32 } else { 65.0 };
                            egui::ScrollArea::both()
                                .id_salt(("plugin_table", plugin_idx, i))
                                .auto_shrink([false, true])
                                .max_height(360.0)
                                .show(ui, |ui| {
                                    let avail = ui.available_width();
                                    let first_col = if num_cols > 1 {
                                        (avail - (num_cols - 1) as f32 * fixed_w).max(80.0)
                                    } else {
                                        avail.max(40.0)
                                    };
                                    let total_w = first_col + (num_cols.saturating_sub(1)) as f32 * fixed_w;
                                    let col_w = |ci: usize| -> f32 {
                                        if ci == 0 { first_col } else { fixed_w }
                                    };

                                    {
                                        let (hr, _) = ui.allocate_exact_size(
                                            egui::vec2(total_w, 22.0), egui::Sense::hover()
                                        );
                                        ui.painter().rect_filled(hr, 0.0, header_bg);
                                        let hc = egui::Color32::from_gray(200);
                                        let yc = hr.center().y;
                                        let mut x = hr.min.x;
                                        for (ci, txt) in headers.iter().enumerate() {
                                            ui.painter().text(
                                                egui::pos2(x + cell_pad.x, yc),
                                                egui::Align2::LEFT_CENTER,
                                                *txt,
                                                egui::FontId::proportional(11.5),
                                                hc,
                                            );
                                            let cw = col_w(ci);
                                            if ci + 1 < num_cols {
                                                ui.painter().line_segment(
                                                    [egui::pos2(x + cw, hr.min.y),
                                                     egui::pos2(x + cw, hr.max.y)],
                                                    egui::Stroke::new(1.0, divider_col),
                                                );
                                            }
                                            x += cw;
                                        }
                                        ui.painter().line_segment(
                                            [hr.left_bottom(), hr.right_bottom()],
                                            egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                                        );
                                    }

                                    let rows = &desc.widgets[rows_start..rows_end];

                                    if rows.is_empty() {
                                        let (ph, _) = ui.allocate_exact_size(
                                            egui::vec2(total_w, 28.0), egui::Sense::hover()
                                        );
                                        ui.painter().text(
                                            egui::pos2(ph.min.x + cell_pad.x, ph.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            "no data",
                                            egui::FontId::proportional(11.0),
                                            egui::Color32::from_gray(90),
                                        );
                                    }

                                    for (row_idx, row) in rows.iter().enumerate() {
                                        let stripe = if row_idx % 2 == 0 { stripe_dark } else { stripe_light };
                                        let cells: Vec<&str> = row.label_str().split('\t').collect();

                                        let heat = if heat_col > 0 && heat_max > heat_min {
                                            let t = ((row.value - heat_min) / (heat_max - heat_min)).clamp(0.0, 1.0) as f32;
                                            let r = (200.0 * (1.0 - t) + 60.0 * t) as u8;
                                            let g = (80.0 * (1.0 - t) + 220.0 * t) as u8;
                                            egui::Color32::from_rgb(r, g, 80)
                                        } else {
                                            egui::Color32::from_gray(200)
                                        };

                                        let (row_rect, row_resp) = ui.allocate_exact_size(
                                            egui::vec2(total_w, 36.0), egui::Sense::hover()
                                        );
                                        ui.painter().rect_filled(row_rect, 0.0, stripe);
                                        if row_resp.hovered() {
                                            ui.painter().rect_filled(
                                                row_rect, 0.0,
                                                egui::Color32::from_rgba_premultiplied(255, 255, 255, 8),
                                            );
                                        }

                                        let mut x = row_rect.min.x;
                                        let mid_y = row_rect.center().y;

                                        for (ci, cell) in cells.iter().enumerate() {
                                            let is_heat = heat_col > 0 && ci + 1 == heat_col;
                                            let color = if is_heat { heat } else { egui::Color32::WHITE };
                                            let cw = col_w(ci);

                                            if cell.contains('\n') {
                                                let mut parts = cell.splitn(2, '\n');
                                                let line1 = parts.next().unwrap_or("");
                                                let line2 = parts.next().unwrap_or("");
                                                ui.painter().text(
                                                    egui::pos2(x + cell_pad.x, mid_y - 7.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    line1,
                                                    egui::FontId::proportional(11.5),
                                                    color,
                                                );
                                                ui.painter().text(
                                                    egui::pos2(x + cell_pad.x, mid_y + 7.0),
                                                    egui::Align2::LEFT_CENTER,
                                                    line2,
                                                    egui::FontId::proportional(11.5),
                                                    color,
                                                );
                                            } else {
                                                ui.painter().text(
                                                    egui::pos2(x + cell_pad.x, mid_y),
                                                    egui::Align2::LEFT_CENTER,
                                                    *cell,
                                                    egui::FontId::proportional(11.5),
                                                    color,
                                                );
                                            }

                                            if ci + 1 < num_cols {
                                                ui.painter().line_segment(
                                                    [egui::pos2(x + cw, row_rect.min.y),
                                                     egui::pos2(x + cw, row_rect.max.y)],
                                                    egui::Stroke::new(1.0, divider_col),
                                                );
                                            }
                                            x += cw;
                                        }

                                        ui.painter().line_segment(
                                            [row_rect.left_bottom(), row_rect.right_bottom()],
                                            egui::Stroke::new(1.0, egui::Color32::from_gray(35)),
                                        );
                                    }
                                });

                            i = rows_end;
                            in_horizontal = false;
                            continue;
                        }
                        WIDGET_TABLE_ROW => {}
                        _ => {}
                    }
                    i += 1;
                }
                let _ = in_horizontal;
            });
        });
}

fn render_inline_widget(
    ui: &mut egui::Ui,
    w: &PluginWidgetDesc,
    handle_action: Option<unsafe extern "C" fn(u32, f64)>,
    handle_text: Option<unsafe extern "C" fn(u32, *const u8, u32)>,
) {
    match w.kind {
        WIDGET_BUTTON => {
            let mut rt = egui::RichText::new(w.label_str());
            if w.flags & WIDGET_FLAG_SMALL != 0  { rt = rt.small(); }
            let color_packed = w.max_value as u32;
            if color_packed != 0 {
                let r = ((color_packed >> 24) & 0xFF) as u8;
                let g = ((color_packed >> 16) & 0xFF) as u8;
                let b = ((color_packed >> 8) & 0xFF) as u8;
                rt = rt.color(egui::Color32::from_rgb(r, g, b));
            }
            if ui.add_enabled(w.is_enabled(), egui::Button::new(rt)).clicked() {
                if let Some(f) = handle_action {
                    safe_action_call(f, w.id, 0.0);
                }
            }
        }
        WIDGET_LABEL => {
            let mut rt = egui::RichText::new(w.label_str());
            if w.flags & WIDGET_FLAG_SMALL != 0  { rt = rt.small(); }
            if w.flags & WIDGET_FLAG_WEAK != 0   { rt = rt.weak(); }
            if w.flags & WIDGET_FLAG_STRONG != 0 { rt = rt.strong(); }
            ui.label(rt);
        }
        WIDGET_CHECKBOX => {
            let mut checked = w.value != 0.0;
            if ui.add_enabled(w.is_enabled(), egui::Checkbox::new(&mut checked, w.label_str())).changed() {
                if let Some(f) = handle_action {
                    safe_action_call(f, w.id, if checked { 1.0 } else { 0.0 });
                }
            }
        }
        WIDGET_DRAG_VALUE => {
            let mut val = w.value;
            let speed = w.min_value.max(1.0);
            if ui.add_enabled(w.is_enabled(), egui::DragValue::new(&mut val).speed(speed)).changed() {
                if let Some(f) = handle_action {
                    safe_action_call(f, w.id, val);
                }
            }
        }
        WIDGET_SEPARATOR => {
            ui.separator();
        }
        WIDGET_COLOR_LABEL => {
            let packed = w.value as u32;
            let r = ((packed >> 24) & 0xFF) as u8;
            let g = ((packed >> 16) & 0xFF) as u8;
            let b = ((packed >> 8) & 0xFF) as u8;
            let mut rt = egui::RichText::new(w.label_str()).color(egui::Color32::from_rgb(r, g, b));
            if w.flags & WIDGET_FLAG_SMALL != 0  { rt = rt.small(); }
            if w.flags & WIDGET_FLAG_STRONG != 0 { rt = rt.strong(); }
            ui.label(rt);
        }
        WIDGET_PROGRESS => {
            let cur = w.value as f32;
            let total = w.min_value as f32;
            let pct = if total > 0.0 { cur / total } else { 0.0 };
            let bar_color = egui::Color32::from_rgb(80, 180, 100);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(80.0, 10.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 3.0, egui::Color32::from_gray(50));
            let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * pct.clamp(0.0, 1.0), rect.height()));
            ui.painter().rect_filled(fill, 3.0, bar_color);
            ui.label(format!("{}/{}", cur as u32, total as u32));
        }
        _ => {}
    }
}

fn safe_action_call(f: unsafe extern "C" fn(u32, f64), widget_id: u32, value: f64) {
    let _ = microseh::try_seh(|| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            unsafe { f(widget_id, value) };
        }))
    });
}

fn safe_text_call(f: unsafe extern "C" fn(u32, *const u8, u32), widget_id: u32, text: &str) {
    let _ = microseh::try_seh(|| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            unsafe { f(widget_id, text.as_ptr(), text.len() as u32) };
        }))
    });
}

static PANEL_VISIBILITY: Mutex<Vec<bool>> = Mutex::new(Vec::new());

pub fn plugin_count() -> usize {
    PLUGINS.lock().map(|p| p.len()).unwrap_or(0)
}

fn format_panic_box(e: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() { (*s).to_owned() }
    else if let Some(s) = e.downcast_ref::<String>() { s.clone() }
    else { "unknown panic payload".to_owned() }
}

macro_rules! for_each_battle_cb {
    ($field:ident, $call:expr) => {{
        let fns: Vec<_> = {
            let plugins = PLUGINS.lock().unwrap();
            plugins.iter()
                .filter(|p| !p.battle_callbacks.is_null())
                .filter_map(|p| unsafe { (*p.battle_callbacks).$field })
                .collect()
        };
        for f in fns {
            match microseh::try_seh(|| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $call(f)))
            }) {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    let message = format!("[veritas::plugin] panic in battle dispatch {}: {}", stringify!($field), format_panic_box(&e));
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| log::error!("{}", message)));
                }
                Err(e) => {
                    let message = format!("[veritas::plugin] SEH in battle dispatch {}: {:?}", stringify!($field), e);
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| log::error!("{}", message)));
                }
            }
        }
    }};
}

pub fn dispatch_battle_begin(game_mode_ptr: usize) {
    for_each_battle_cb!(on_battle_begin, |f: unsafe extern "C" fn(usize)| unsafe { f(game_mode_ptr) });
}

pub fn dispatch_battle_end() {
    for_each_battle_cb!(on_battle_end, |f: unsafe extern "C" fn()| unsafe { f() });
}

pub fn dispatch_battle_end_with_result(total_damage: f64, action_value: f64, turn_count: u32, cycle: u32) {
    for_each_battle_cb!(on_battle_end_with_result, |f: unsafe extern "C" fn(f64, f64, u32, u32)| unsafe { f(total_damage, action_value, turn_count, cycle) });
}

pub fn dispatch_on_damage(attacker_uid: u32, damage: f64) {
    for_each_battle_cb!(on_damage, |f: unsafe extern "C" fn(u32, f64)| unsafe { f(attacker_uid, damage) });
}

pub fn dispatch_on_set_lineup(instance_ptr: usize, lineup_data_ptr: usize) {
    for_each_battle_cb!(on_set_lineup, |f: unsafe extern "C" fn(usize, usize)| unsafe { f(instance_ptr, lineup_data_ptr) });
}

pub fn dispatch_on_turn_begin(game_mode_ptr: usize) {
    for_each_battle_cb!(on_turn_begin, |f: unsafe extern "C" fn(usize)| unsafe { f(game_mode_ptr) });
}

pub fn dispatch_on_init_enemy(component_ptr: usize) {
    for_each_battle_cb!(on_init_enemy, |f: unsafe extern "C" fn(usize)| unsafe { f(component_ptr) });
}

pub fn dispatch_on_use_skill(component_ptr: usize, skill_index: i32, extra: i32) {
    for_each_battle_cb!(on_use_skill, |f: unsafe extern "C" fn(usize, i32, i32)| unsafe { f(component_ptr, skill_index, extra) });
}


