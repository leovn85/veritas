use anyhow::{Result, anyhow};
use il2cpp_runtime::{Il2CppObject, types::List, get_cached_class, api::{il2cpp_class_get_fields, il2cpp_field_get_type, il2cpp_field_get_offset}};
use il2cpp_runtime::prelude::*;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug)]
struct RelicAffixOffsets {
	count: usize,
	step: usize,
	property_id: usize,
}

static RELIC_AFFIX_OFFSETS: OnceLock<RelicAffixOffsets> = OnceLock::new();

unsafe fn resolve_relic_affix_offsets() -> Result<RelicAffixOffsets> {
	// 1. Lấy Class RelicItemData
	let relic_data_class = get_cached_class("RPG.Client.RelicItemData")?;

	// 2. Lấy Method _GetAvatarPropertyTypeByRelicAffix
	let method = relic_data_class.find_method("_GetAvatarPropertyTypeByRelicAffix", &["*"])?;
	
	// Tham số đầu tiên chính là Class RelicAffix bị obfuscate
	let affix_class = method.arg(0).class();

	let mut uint_offsets = Vec::new();
	let field_iter: *const c_void = std::ptr::null();
	
	// 3. Quét toàn bộ field của Class này
	loop {
		let field = il2cpp_class_get_fields(affix_class, &field_iter);
		if field.0.is_null() { break; }

		let f_type = il2cpp_field_get_type(field);
		
		// 4. Tìm các field có kiểu là uint (System.UInt32)
		if f_type.name() == "System.UInt32" {
			uint_offsets.push(il2cpp_field_get_offset(field) as usize);
		}
	}

	// 5. Đảm bảo thứ tự tăng dần (0x18 -> property_id, 0x1C -> step, 0x20 -> count)
	if uint_offsets.len() >= 3 {
		uint_offsets.sort(); 
		Ok(RelicAffixOffsets {
			count: uint_offsets[0],
			step: uint_offsets[1],
			property_id: uint_offsets[2],
		})
	} else {
		Err(anyhow!("Failed to dynamically resolve RelicAffix fields!"))
	}
}

unsafe fn get_relic_affix_offsets() -> Result<RelicAffixOffsets> {
	if let Some(offsets) = RELIC_AFFIX_OFFSETS.get() {
		return Ok(*offsets);
	}
	let offsets = unsafe { resolve_relic_affix_offsets()? };
	let _ = RELIC_AFFIX_OFFSETS.set(offsets);
	Ok(offsets)
}

//use crate::subscribers::subscribe_function;
use crate::kreide::types::{
	RPG_Client_EquipmentItemData, RPG_Client_InventoryModule, RPG_Client_RelicItemData, RPG_Client_TextmapStatic, RPG_GameCore_AvatarPropertyExcelTable, RPG_GameCore_FixPoint, RPG_GameCore_GamePlayStatic, RPG_GameCore_RelicBaseTypeExcelTable, RPG_GameCore_RelicSetConfigExcelTable, RPG_GameCore_RelicSubAffixConfigExcelTable
};
use crate::models::misc::{LightCone, Relic, RelicMainStat, RelicRolls, RelicSubstat, ReliquaryLightCone, ReliquaryRelic};
use crate::relic_utils::{calc_initial_rolls, get_light_cones, get_relics, pick_low_mid_high/*, write_light_cones_to_json, write_relics_to_json*/};

retour::static_detour! {
	static _UpdateRelics_Detour: unsafe extern "C" fn(RPG_Client_InventoryModule, List, bool);
	static sync_relic_Detour: unsafe extern "C" fn(RPG_Client_RelicItemData, *const c_void);
	static sync_equipment_Detour: unsafe extern "C" fn(RPG_Client_EquipmentItemData, *const c_void);
	static _UpdateEquipments_Detour: unsafe extern "C" fn(RPG_Client_InventoryModule, List, bool);
}

impl Into<f64> for RPG_GameCore_FixPoint {
	fn into(self) -> f64 {
		const FLOAT_CONVERSION_CONSTANT: f64 = 1.0 / 4294967296.0;
		let raw_value = self.m_rawValue;
		let hi = ((raw_value as u64 & 0xFFFFFFFF00000000) >> 32) as u32;
		let lo = (raw_value as u64 & 0x00000000FFFFFFFF) as u32;
		hi as f64 + lo as f64 * FLOAT_CONVERSION_CONSTANT
	}
}

static ARE_RELICS_INITIALIZED: OnceLock<bool> = OnceLock::new();
static ARE_LIGHT_CONES_INITIALIZED: OnceLock<bool> = OnceLock::new();

fn pending_relic_updates() -> &'static Mutex<Vec<ReliquaryRelic>> {
	static PENDING_RELIC_UPDATES: OnceLock<Mutex<Vec<ReliquaryRelic>>> = OnceLock::new();
	PENDING_RELIC_UPDATES.get_or_init(|| Mutex::new(Vec::new()))
}

fn pending_light_cone_updates() -> &'static Mutex<Vec<ReliquaryLightCone>> {
	static PENDING_LIGHT_CONE_UPDATES: OnceLock<Mutex<Vec<ReliquaryLightCone>>> = OnceLock::new();
	PENDING_LIGHT_CONE_UPDATES.get_or_init(|| Mutex::new(Vec::new()))
}

fn update_relics(this: RPG_Client_InventoryModule, _list: List, flag: bool) {
	let initialized = ARE_RELICS_INITIALIZED.get().copied().unwrap_or(false);
	unsafe { _UpdateRelics_Detour.call(this, _list, flag) };

	if initialized {
		let pending = pending_relic_updates();
		let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
		if !guard.is_empty() {
			let live_relics = std::mem::take(&mut *guard);
			drop(guard);
			crate::relic_server::send_live_relic_update(live_relics);
		}
	} else {
		pending_relic_updates().lock().unwrap_or_else(|e| e.into_inner()).clear();
	}

	//let _ = write_relics_to_json("relics.json");
	ARE_RELICS_INITIALIZED.get_or_init(|| true);
}

fn update_equipments(this: RPG_Client_InventoryModule, _list: List, flag: bool) {
	let initialized = ARE_LIGHT_CONES_INITIALIZED.get().copied().unwrap_or(false);
	unsafe { _UpdateEquipments_Detour.call(this, _list, flag) };

	if initialized {
		let pending = pending_light_cone_updates();
		let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
		if !guard.is_empty() {
			let live_light_cones = std::mem::take(&mut *guard);
			drop(guard);
			crate::relic_server::send_live_light_cone_update(live_light_cones);
		}
	} else {
		pending_light_cone_updates().lock().unwrap_or_else(|e| e.into_inner()).clear();
	}

	//let _ = write_light_cones_to_json("light_cones.json");
	ARE_LIGHT_CONES_INITIALIZED.get_or_init(|| true);
}

fn sync_equipment(this: RPG_Client_EquipmentItemData, packet: *const c_void) {
	unsafe { sync_equipment_Detour.call(this, packet) };
	if let Ok(live_light_cone) = process_equipment_data(this) {
		if ARE_LIGHT_CONES_INITIALIZED.get().copied().unwrap_or(false) {
			pending_light_cone_updates().lock().unwrap_or_else(|e| e.into_inner()).push(live_light_cone);
		}
	}
}

fn process_equipment_data(this: RPG_Client_EquipmentItemData) -> Result<ReliquaryLightCone> {
	let uid = unsafe { this.as_base().get_UID()? };
	let location = unsafe { this.get_BelongAvatarID()? };
	let lock = unsafe { this.get_IsProtected()? };
	let rank = (*this._Rank()?).0;
	let level = unsafe { this.get_Level()? };
	let promotion = unsafe { this.get_Promotion()? };
	let equipment_row = unsafe { this.get_EquipmentRow()? };
	let name = unsafe { RPG_Client_TextmapStatic::get_text(&*equipment_row.EquipmentName()?, std::ptr::null())? };
	let id = (*equipment_row.EquipmentID()?).0;
	
	let light_cone = LightCone {
		id: id.to_string(),
		name: name.to_string(),
		level: level as u32,
		promotion: promotion as u32,
		rank: rank as u32,
		equipped_by: if location > 0 { location.to_string() } else { String::new() },
		lock,
		uid: uid.to_string(),
	};

	let live_light_cone = ReliquaryLightCone::from(&light_cone);
	get_light_cones().write().insert(uid.to_string(), light_cone);
	Ok(live_light_cone)
}

fn sync_relic(this: RPG_Client_RelicItemData, packet: *const c_void) {
	unsafe { sync_relic_Detour.call(this, packet) };
	if let Ok(live_relic) = process_relic_data(this) {
		if ARE_RELICS_INITIALIZED.get().copied().unwrap_or(false) {
			pending_relic_updates().lock().unwrap_or_else(|e| e.into_inner()).push(live_relic);
		}
	}
}

#[il2cpp_ref_type("System.Object")]
pub struct SystemObjectDummy;

fn process_relic_data(this: RPG_Client_RelicItemData) -> Result<ReliquaryRelic> {
	unsafe 
	{
		let relic_row = this.get_RelicRow()?;
		let set_id = (*relic_row.SetID()?).0;
		let location = this.get_BelongAvatarID()?;
		let lock = this.get_IsProtected()?;
		let discard = this.get_IsDiscard()?;
		let uid = this.as_base().get_UID()?;
		let rarity = (*relic_row.Rarity()?) as u32;
		let level = this.get_Level()?;
		let relic_set_config_data = RPG_GameCore_RelicSetConfigExcelTable::GetData(set_id)?;
		let relic_set_name = RPG_Client_TextmapStatic::get_text(&*relic_set_config_data.SetName()?, std::ptr::null())?;
		let main_affix_property = this.get_MainAffixPropertyType()?;
		let main_row_data = RPG_GameCore_AvatarPropertyExcelTable::GetData(main_affix_property)?;
		let main_stat_name = RPG_Client_TextmapStatic::get_text(&*main_row_data.PropertyName()?, std::ptr::null())?;
		let relic_type_row = RPG_GameCore_RelicBaseTypeExcelTable::GetData(*relic_row.Type()?)?;
		let slot_name = RPG_Client_TextmapStatic::get_text(&*relic_type_row.BaseTypeText()?, std::ptr::null())?;

		//let mut substats = Vec::new();
		//let mut total_count: i32 = 0;
		
		// Lấy Array thay vì mảng cụ thể
		//let sub_affix_list = this.get_SubAffixList()?;
		
		// Gọi hàm động để lấy Offsets
		//let offsets = unsafe { get_relic_affix_offsets()? };

		/* for i in 0..sub_affix_list.len() {
			//let affix_obj: Il2CppObject = sub_affix_list.get(i);
			let affix_obj: &SystemObjectDummy = sub_affix_list.get(i);
			let ptr = affix_obj.as_ptr() as *const u8;
			
			// Đọc data bằng Dynamic Offsets
			let count = unsafe { *(ptr.add(offsets.count) as *const u32) } as i32;
			let step = unsafe { *(ptr.add(offsets.step) as *const u32) } as i32;
			let affix_id = unsafe { *(ptr.add(offsets.property_id) as *const u32) };

			let sub_property = this._GetPropertyTypeBySubAffixID(affix_id)?;
			let sub_row_data = RPG_GameCore_AvatarPropertyExcelTable::GetData(sub_property)?;
			let property_name = RPG_Client_TextmapStatic::get_text(&*sub_row_data.PropertyName()?, std::ptr::null())?.to_string();

			total_count = total_count.saturating_add(count);

			let relic_sub_affix_config = RPG_GameCore_RelicSubAffixConfigExcelTable::GetData((*relic_row.SubAffixGroup()?).0, affix_id)?;
			let mut value: f64 = RPG_GameCore_GamePlayStatic::CalcRelicSubAffixValue(*relic_sub_affix_config.BaseValue()?, *relic_sub_affix_config.StepValue()?, count as u32, step as u32)?.into();
			
			let mut stat_name = property_name;
			if value < 1.0 { stat_name.push('%'); value *= 100.0; }

			let (low, mid, high) = pick_low_mid_high(step, count);
			substats.push(RelicSubstat { 
				stat: stat_name, 
				value, 
				rolls: RelicRolls { high, mid, low }, 
				added_rolls: (count - 1).max(0),
				raw_count: count, // TRUYỀN VÀO ĐÂY
				raw_step: step,	  // TRUYỀN VÀO ĐÂY
			});
		} */
		
		let parse_affix_array = |array: Il2CppArray| -> Result<Option<Vec<crate::models::misc::Substat>>> {
			if array.as_ptr().is_null() || array.len() == 0 {
				return Ok(None); // An toàn: Không bị crash nếu array bị NULL
			}
			
			let mut subs = Vec::new();
			let offsets = get_relic_affix_offsets()?;

			for i in 0..array.len() {
				let affix_obj: &SystemObjectDummy = array.get(i);
				let ptr = affix_obj.as_ptr() as *const u8;
				
				let count = *(ptr.add(offsets.count) as *const u32);
				let step = *(ptr.add(offsets.step) as *const u32);
				let affix_id = *(ptr.add(offsets.property_id) as *const u32);
				
				if affix_id == 0 {
                    continue;
                }

				// println!("Count     | Offset: {:<4} | Value: {}", offsets.count, count);
				// println!("Step      | Offset: {:<4} | Value: {}", offsets.step, step);
				// println!("Affix ID  | Offset: {:<4} | Value: {}", offsets.property_id, affix_id);

				let sub_property = this._GetPropertyTypeBySubAffixID(affix_id)?;
				let sub_row_data = RPG_GameCore_AvatarPropertyExcelTable::GetData(sub_property)?;
				
				if sub_row_data.0.is_null() {
                    continue;
                }
				
				let property_name = RPG_Client_TextmapStatic::get_text(&*sub_row_data.PropertyName()?, std::ptr::null())?.to_string();
				
				//println!("property_name | Value: {}", property_name);

				let relic_sub_affix_config = RPG_GameCore_RelicSubAffixConfigExcelTable::GetData((*relic_row.SubAffixGroup()?).0, affix_id)?;
				
				if relic_sub_affix_config.0.is_null() {
                    continue;
                }
				
				let mut value: f64 = RPG_GameCore_GamePlayStatic::CalcRelicSubAffixValue(*relic_sub_affix_config.BaseValue()?, *relic_sub_affix_config.StepValue()?, count, step)?.into();
				
				//println!("value from CalcRelicSubAffixValue | Value: {}", value);
				
				let mut key = property_name;
				if value < 1.0 { key.push('_'); value *= 100.0; } // Ví dụ: "CRIT Rate" -> "CRIT Rate_"

				subs.push(crate::models::misc::Substat {
					key,
					value: value as f64, 
					count: count as u32,
					step: step as u32,
				});
			}
			Ok(Some(subs))
		};

		// --- 2. ĐỌC CÁC MẢNG DỮ LIỆU ---
		
		// A. Mảng Reroll và Preview (Dùng trực tiếp cấu trúc Substat)
		let reroll_substats = parse_affix_array(this.get_ReforgeSubAffixes()?)?;
		let preview_substats = parse_affix_array(this.get_PreviewSubAffixList()?)?;

		// B. Mảng Main Substats (Cần chuyển sang RelicSubstat để vẽ UI)
		let raw_main_substats = parse_affix_array(this.get_SubAffixList()?)?.unwrap_or_default();
		
		let mut ui_substats = Vec::new();
		let mut total_count: u32 = 0;

		for sub in raw_main_substats {
			total_count = total_count.saturating_add(sub.count as u32);
			let (low, mid, high) = pick_low_mid_high(sub.step as u32, sub.count as u32);
			
			ui_substats.push(RelicSubstat { 
				stat: sub.key, 
				value: sub.value as f64, 
				rolls: RelicRolls { high, mid, low }, 
				added_rolls: (sub.count as u32 - 1).max(0),
				raw_count: sub.count as u32, 
				raw_step: sub.step as u32,
			});
		}

		// --- 3. LẮP RÁP THÀNH RELIC CUỐI CÙNG ---
		let initial_rolls = if total_count > 0 { calc_initial_rolls(level as u32, total_count as u32) } else { 0 };
		let mut main_value: f64 = (this.GetMainAffixPropertyValue()?).into();
		let main_stat = main_stat_name.to_string();
		if main_value < 1.0 { main_value *= 100.0; }

		let relic = Relic {
			part: slot_name.to_string(), 
			set_id: set_id.to_string(), 
			set: relic_set_name.to_string(), 
			enhance: level as u32, 
			grade: rarity, 
			main: RelicMainStat { stat: main_stat, value: main_value }, 
			substats: ui_substats, // <-- Đưa mảng UI vào đây
			reroll_substats,	   // <-- Đưa mảng Reroll vào đây
			preview_substats,	   // <-- Đưa mảng Preview vào đây
			equipped_by: if location > 0 { location.to_string() } else { String::new() }, 
			verified: true, 
			id: uid.to_string(), 
			age_index: uid, 
			initial_rolls, 
			lock, 
			discard,
		};
		
		get_relics().write().insert(uid.to_string(), relic.clone());
		Ok(ReliquaryRelic::from(&relic))
	}
}

pub fn subscribe() -> Result<()> {
	unsafe {
		let class_relic = RPG_Client_RelicItemData::get_class_static()?;
		subscribe_function!(sync_relic_Detour, class_relic.find_method("Sync", &["*"])?.va(), sync_relic)?;

		let class_inventory = RPG_Client_InventoryModule::get_class_static()?;
		subscribe_function!(_UpdateRelics_Detour, class_inventory.find_method("_UpdateRelics", &["*", "bool"])?.va(), update_relics)?;
		subscribe_function!(_UpdateEquipments_Detour, class_inventory.find_method("_UpdateEquipments", &["*", "bool"])?.va(), update_equipments)?;

		let class_equip = RPG_Client_EquipmentItemData::get_class_static()?;
		subscribe_function!(sync_equipment_Detour, class_equip.find_method("Sync", &["*"])?.va(), sync_equipment)?;
	}
	Ok(())
}