use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;
use chrono::Local;
use std::fs::File;
use std::io::Write;
use serde_json::json;

use crate::models::misc::{LightCone, Relic, ReliquaryLightCone, ReliquaryRelic, FribbelsArchive, FribbelsMetadata};
use crate::kreide::helpers::dump_fribbels_characters;

pub fn get_relics() -> &'static RwLock<HashMap<String, Relic>> {
    static RELICS: OnceLock<RwLock<HashMap<String, Relic>>> = OnceLock::new();
    RELICS.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn get_light_cones() -> &'static RwLock<HashMap<String, LightCone>> {
    static LIGHT_CONES: OnceLock<RwLock<HashMap<String, LightCone>>> = OnceLock::new();
    LIGHT_CONES.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn calc_initial_rolls(level: u32, total_rolls: u32) -> u32 {
    //total_rolls - level.div_floor(3)
	total_rolls - (level / 3)
}

pub fn solve_low_mid_high(step: u32, count: u32) -> Vec<(u32, u32, u32)> {
    if step < 0 || count < 0 {
        return Vec::new();
    }

    // 0*low + 1*mid + 2*high = step
    // low + mid + high = count
    // mid = step - 2*high
    // low = count - step + high
    let high_min = (step - count).max(0);
    let high_max = step / 2;

    if high_min > high_max {
        return Vec::new();
    }

    (high_min..=high_max)
        .map(|high| {
            let mid = step - 2 * high;
            let low = count - step + high;
            (low, mid, high)
        })
        .filter(|(low, mid, high)| *low >= 0 && *mid >= 0 && *high >= 0)
        .collect()
}

pub fn pick_low_mid_high(step: u32, count: u32) -> (u32, u32, u32) {
    solve_low_mid_high(step, count)
        .last()
        .copied()
        .unwrap_or((0, 0, 0))
}

pub fn write_relics_to_json(path: &str) -> Result<()> {
    let relics_map = get_relics().read();
    let relics: Vec<ReliquaryRelic> = relics_map
        .values()
        .map(|relic| ReliquaryRelic::from(relic))
        .collect();

    let json_obj = serde_json::json!({
        "relics": relics
    });

    let json_str = serde_json::to_string_pretty(&json_obj)?;
    std::fs::write(path, json_str)?;

    log::info!("Wrote {} relics to {}", relics.len(), path);
    Ok(())
}

pub fn get_relics_snapshot() -> Vec<Relic> {
    let relics_map = get_relics().read();
    relics_map.values().cloned().collect()
}

pub fn write_light_cones_to_json(path: &str) -> Result<()> {
    let light_cones_map = get_light_cones().read();
    let light_cones: Vec<ReliquaryLightCone> = light_cones_map
        .values()
        .map(|lc| ReliquaryLightCone::from(lc))
        .collect();

    let json_obj = serde_json::json!({
        "light_cones": light_cones
    });

    let json_str = serde_json::to_string_pretty(&json_obj)?;
    std::fs::write(path, json_str)?;

    log::info!("Wrote {} light cones to {}", light_cones.len(), path);
    Ok(())
}

#[allow(dead_code)]
pub fn get_light_cones_snapshot() -> Vec<LightCone> {
    let light_cones_map = get_light_cones().read();
    light_cones_map.values().cloned().collect()
}

pub struct RelicMappingInfo {
    pub relic_id: u32,
    pub relic_type: i32,
}

/// Đọc trực tiếp từ RAM để tạo bảng Map: (SetID, Rarity, SlotName) -> (RelicID, RelicType)
pub unsafe fn build_relic_id_mapping() -> anyhow::Result<HashMap<(u32, u32, String), RelicMappingInfo>> {
    let mut map = HashMap::new();
    let dict_ptr = crate::kreide::types::RPG_GameCore_RelicConfigExcelTable::get_dataDict()?;
    let rows = crate::kreide::helpers::extract_rows_from_dict_ram(dict_ptr).unwrap_or_default();

    for row_ptr in rows {
        let row = crate::kreide::types::RPG_GameCore_RelicConfigRow(row_ptr as _);
        
        let id = (*row.ID()?).0;
        let set_id = (*row.SetID()?).0;
        let rarity = *row.Rarity()? as u32; // CombatPowerRelicRarity1 = 0 -> Rarity 1
        let type_enum_val = *row.Type()? as i32;
        
        // Lấy tên Slot đã được Localize (VD: "Head", "Hands", hoặc "Nón", "Găng")
        let safe_type = microseh::try_seh(|| crate::kreide::types::RPG_GameCore_RelicBaseTypeExcelTable::GetData(*row.Type()?));
        let mut slot_name = String::new();
        if let Ok(Ok(type_row)) = safe_type {
            if !type_row.0.is_null() {
                let safe_text_id = microseh::try_seh(|| type_row.BaseTypeText());
                if let Ok(Ok(text_id)) = safe_text_id {
                    let safe_text = microseh::try_seh(|| crate::kreide::helpers::get_textmap_content(&*text_id));
                    if let Ok(Ok(name)) = safe_text {
                        slot_name = crate::kreide::helpers::sanitize_entity_name(name);
                    }
                }
            }
        }
        
        if !slot_name.is_empty() {
            map.insert((set_id, rarity, slot_name), RelicMappingInfo {
                relic_id: id,
                relic_type: type_enum_val,
            });
        }
    }
    Ok(map)
}

/// Hàm chính được gọi khi bấm nút Dump
pub fn dump_and_convert_data() -> anyhow::Result<()> {
    log::info!("=== BEGIN FRIBBELS DUMP ===");

    // 1. Lấy Relics & Light Cones
    let relics: Vec<ReliquaryRelic> = get_relics_snapshot()
        .iter().map(|r| ReliquaryRelic::from(r)).collect();
    log::info!("Dumped {} Relics from snapshot.", relics.len());

    let light_cones: Vec<ReliquaryLightCone> = get_light_cones_snapshot()
        .iter().map(|lc| ReliquaryLightCone::from(lc)).collect();
    log::info!("Dumped {} Light Cones from snapshot.", light_cones.len());

    // 2. Gọi hàm lấy Characters, UID, Name
    let (characters, player_uid, tb_meta) = unsafe { 
        crate::kreide::helpers::dump_fribbels_characters().unwrap_or_else(|e| {
            log::error!("Failed to dump characters: {}", e);
            (Vec::new(), 0, "Unknown (Stelle)".to_string())
        }) 
    };

    // 3. Tạo file JSON
    let archive = FribbelsArchive {
        source: "reliquary_archiver".to_string(),
        build: "0.14.0".to_string(),
        version: 4,
        metadata: FribbelsMetadata { 
            uid: player_uid, 
            trailblazer: tb_meta 
        },
        light_cones,
        relics,
        characters,
    };

    let now_str = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let archive_filename = format!("archive_output-{}.json", now_str);
    let archive_json = serde_json::to_string_pretty(&archive)?;
    std::fs::write(&archive_filename, &archive_json)?;
    log::info!("Created Fribbels file successfully: {}", archive_filename);

    // 4. CONVERT SANG CONFIG.JSON CHO PRIVATE SERVER (Nếu bạn vẫn đang xài đoạn này)
    log::info!("Generating Private Server config...");
    generate_private_server_config(&archive)?;
    
    log::info!("=== END FRIBBELS DUMP ===");
    Ok(())
}

fn generate_private_server_config(archive: &FribbelsArchive) -> anyhow::Result<()> {
    // 1. Lấy bảng Map Relic ID từ RAM
    let relic_lookup = unsafe { build_relic_id_mapping().unwrap_or_default() };

    // 2. Map Sub Affix
    let sub_affix_map: HashMap<&str, u32> = HashMap::from([
        ("HP", 1), ("ATK", 2), ("DEF", 3), ("HP_", 4), ("ATK_", 5), ("DEF_", 6),
        ("SPD", 7), ("CRIT Rate_", 8), ("CRIT DMG_", 9), ("Effect Hit Rate_", 10),
        ("Effect RES_", 11), ("Break Effect_", 12)
    ]);

    // 3. Map Main Affix dựa trên Relic Type (1=HEAD, 2=HAND, 3=BODY, 4=FOOT, 5=NECK, 6=OBJECT)
    let get_main_affix = |relic_type: i32, main: &str| -> u32 {
        match relic_type {
            1 | 2 => 1,
            3 => match main { "HP"=>1, "ATK"=>2, "DEF"=>3, "CRIT Rate"=>4, "CRIT DMG"=>5, "Outgoing Healing Boost"=>6, "Effect Hit Rate"=>7, _=>1 },
            4 => match main { "HP"=>1, "ATK"=>2, "DEF"=>3, "SPD"=>4, _=>1 },
            5 => match main { "HP"=>1, "ATK"=>2, "DEF"=>3, "Physical DMG Boost"=>4, "Fire DMG Boost"=>5, "Ice DMG Boost"=>6, "Lightning DMG Boost"=>7, "Wind DMG Boost"=>8, "Quantum DMG Boost"=>9, "Imaginary DMG Boost"=>10, _=>1 },
            6 => match main { "Break Effect"=>1, "Energy Regeneration Rate"=>2, "HP"=>3, "ATK"=>4, "DEF"=>5, _=>1 },
            _ => 1
        }
    };

    // --- Gom nhóm dữ liệu ---
    let mut lc_map = HashMap::new();
    for lc in &archive.light_cones {
        if !lc.location.is_empty() {
            lc_map.insert(lc.location.clone(), lc.clone());
        }
    }

    let mut relic_map: HashMap<String, Vec<&ReliquaryRelic>> = HashMap::new();
    for r in &archive.relics {
        if !r.location.is_empty() {
            relic_map.entry(r.location.clone()).or_default().push(r);
        }
    }

    let mut avatar_configs = Vec::new();

    for char in &archive.characters {
        let char_id = &char.id;

        // Bắt buộc phải có Light Cone
        let Some(lc) = lc_map.get(char_id) else { continue };

        let mut char_relics = Vec::new();
        if let Some(equipped_relics) = relic_map.get(char_id) {
            for r in equipped_relics {
                let set_id_u32 = r.set_id.parse::<u32>().unwrap_or(0);
                
                // Tra cứu Relic ID và Type từ RAM
                let mapping_info = relic_lookup.get(&(set_id_u32, r.rarity, r.slot.clone()));
                
                if let Some(info) = mapping_info {
                    let main_id = get_main_affix(info.relic_type, &r.mainstat);
                    
                    let mut sub_strs = Vec::new();
                    for sub in &r.substats {
                        if let Some(sub_id) = sub_affix_map.get(sub.key.as_str()) {
                            sub_strs.push(format!("{}:{}:{}", sub_id, sub.count, sub.step));
                        }
                    }
                    
                    let sub_str = sub_strs.join(",");
                    let relic_format = format!("{},{},{},{},{}", info.relic_id, r.level, main_id, r.substats.len(), sub_str);
                    
                    // Lưu kèm relic_type để lát nữa sort
                    char_relics.push((info.relic_id, relic_format));
                } else {
                    log::warn!("Could not find relic_id for set_id: {}, rarity: {}, slot: {}", r.set_id, r.rarity, r.slot);
                }
            }
        }

        if char_relics.len() != 6 { continue; } // Yêu cầu đủ 6 món

        // Sort relics theo thứ tự chuẩn: Head(1) -> Hand(2) -> Body(3) -> Feet(4) -> Sphere(5) -> Rope(6)
        char_relics.sort_by_key(|k| k.0);

        let final_relic_strings: Vec<String> = char_relics.into_iter().map(|x| x.1).collect();

        avatar_configs.push(json!({
            "name": char.name,
            "id": char.id.parse::<u32>().unwrap_or(1001),
            "hp": 100, "sp": 50,
            "level": char.level,
            "promotion": char.ascension,
            "rank": char.eidolon,
            "lightcone": {
                "id": lc.id.parse::<u32>().unwrap_or(0),
                "rank": lc.superimposition,
                "level": lc.level,
                "promotion": lc.ascension
            },
            "relics": final_relic_strings,
            "use_technique": true
        }));
    }

    let final_data = json!({
        "avatar_config": avatar_configs,
        "battle_config": {
            "battle_id": 1,
            "stage_id": 1052086,
            "cycle_count": 30,
            "monster_wave": [[4035010]],
            "monster_level": 82,
            "blessings": []
        }
    });

    std::fs::write("config.json", serde_json::to_string_pretty(&final_data)?)?;
    log::info!("Created config.json for Private Server");

    Ok(())
}