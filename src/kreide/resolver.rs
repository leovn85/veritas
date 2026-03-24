// kreide/resolver.rs
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use anyhow::{Context, Result, anyhow};

use crate::kreide::il2cpp::api::*;
use crate::kreide::il2cpp::get_cached_class;


// Biến lưu trữ Offset sau khi đã tìm ra
static DAMAGE_FIELD_OFFSET: LazyLock<Mutex<Option<usize>>> = LazyLock::new(|| Mutex::new(None));
// Biến lưu trữ các offset tiềm năng để đối chiếu chéo giữa các đòn đánh
static DAMAGE_CANDIDATES: LazyLock<Mutex<Option<HashSet<usize>>>> = LazyLock::new(|| Mutex::new(None));

static INTERSECTION_STAGNANT_COUNT: LazyLock<Mutex<u32>> = LazyLock::new(|| Mutex::new(0));

// Map lưu trữ: Tên Tiếng Anh (Key) -> Tên thật trong Game (Value)
static DYNAMIC_REGISTRY: LazyLock<Mutex<HashMap<&'static str, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn get_dynamic_name(key: &str) -> String {
    DYNAMIC_REGISTRY.lock().unwrap().get(key).cloned().unwrap_or_else(|| {
        log::error!("[Resolver] LỖI: Không tìm thấy key '{}' trong Registry!", key);
        key.to_string() // Fallback
    })
}

fn register(key: &'static str, value: String) {
    log::info!("[Resolver] Đã phân giải: {} -> {}", key, value);
    DYNAMIC_REGISTRY.lock().unwrap().insert(key, value);
}

pub fn resolve_all() -> Result<()> {
    log::info!("[Resolver] Bắt đầu quét IL2CPP để phân giải tên động...");

    let domain = il2cpp_domain_get();

    // =========================================================================
    // 1. Phân giải EntityDefeatedEvent (FGFFLOAEKKA)
    // Dấu vết: Tham số đầu tiên của hàm _MakeLimboEntityDie trong TurnBasedGameMode
    // =========================================================================
    let game_mode_class = get_cached_class("RPG.GameCore.TurnBasedGameMode")?;
    let limbo_method = game_mode_class.methods().into_iter()
        .find(|m| m.name() == "_MakeLimboEntityDie")
        .context("Không tìm thấy hàm _MakeLimboEntityDie")?;
    
    let entity_defeated_class = limbo_method.arg(0).class();
    register("EntityDefeatedEvent", entity_defeated_class.name().to_string());

    // Tự động tìm 2 Field "GameEntity" (Killer và Victim) dựa vào kiểu dữ liệu
    // Theo dump, offset 0x10 là Killer, 0x18 là Victim. Ta lấy theo thứ tự xuất hiện.
    let mut entity_fields = Vec::new();
    let iter = std::ptr::null();
    loop {
        let field = unsafe { crate::kreide::il2cpp::api::il2cpp_class_get_fields(entity_defeated_class, &iter) };
        if field.0 == 0 { break; }
        let type_name = unsafe { crate::kreide::il2cpp::api::il2cpp_type_get_name(crate::kreide::il2cpp::api::il2cpp_field_get_type(field)) };
        let type_str = unsafe { crate::kreide::il2cpp::util::cstr_to_str(type_name) };
        if type_str == "RPG.GameCore.GameEntity" {
            entity_fields.push(field.name().to_string());
        }
    }
    if entity_fields.len() >= 2 {
        register("EntityDefeated_KillerField", entity_fields[0].clone());
        register("EntityDefeated_VictimField", entity_fields[1].clone());
    }

    // =========================================================================
    // 2. Phân giải Combo/Insert Skill (FHPFLNJLDHP và OOMAKCLAFOH)
    // Dấu vết: _IsHoldupLimboForWaitingInsertAbilityDone(GameEntity, InsertSkillContext)
    // =========================================================================
    let holdup_method = game_mode_class.methods().into_iter()
        .find(|m| m.name() == "_IsHoldupLimboForWaitingInsertAbilityDone")
        .context("Không tìm thấy hàm _IsHoldupLimboForWaitingInsertAbilityDone")?;
    
    let insert_skill_ctx_class = holdup_method.arg(1).class();
    register("InsertSkillContext", insert_skill_ctx_class.name().to_string());

    // Tìm hàm truyền TurnBasedGameMode vào InsertSkillContext (Chính là hàm HFKBBMIDMOJ)
    for method in insert_skill_ctx_class.methods() {
        if method.args_cnt() == 1 && method.arg(0).formatted_name() == "RPG.GameCore.TurnBasedGameMode" {
            register("InsertSkillMethod", method.name().to_string());
            break;
        }
    }

    // =========================================================================
    // 3. Phân giải Hàm Sát Thương (Damage Method: LCKBMHEANKL) và Event (OHFGNONJNIG)
    // Dấu vết: Hàm có 10 tham số, TaskContext, DamageByAttackProperty, ...
    // =========================================================================
    let mut found_damage = false;
    for assembly in domain.assemblies() {
        if found_damage { break; }
        let image = il2cpp_assembly_get_image(assembly);
        
        // Tối ưu: Chỉ quét Assembly-CSharp
        let img_name = unsafe { crate::kreide::il2cpp::util::cstr_to_str(crate::kreide::il2cpp::api::il2cpp_image_get_name(image)) };
        if img_name != "Assembly-CSharp.dll" && img_name != "Assembly-CSharp" { continue; }

        for class in image.classes() {
            if found_damage { break; }
            for method in class.methods() {
                if method.args_cnt() == 10 {
                    let arg0 = method.arg(0).formatted_name();
                    let arg1 = method.arg(1).formatted_name();
                    let arg3 = method.arg(3).formatted_name();
                    let arg8 = method.arg(8).formatted_name(); // bool flag
                    
                    if arg0 == "RPG.GameCore.TaskContext" 
                        && arg1 == "RPG.GameCore.DamageByAttackProperty" 
                        && arg3 == "RPG.GameCore.TurnBasedAbilityComponent"
                        && arg8 == "bool"
                    {
                        register("DamageClass", class.name().to_string());
                        register("DamageMethod", method.name().to_string());
                        register("DamagePropertyEvent", method.arg(2).class().name().to_string());
                        register("DamageArg9", method.arg(9).class().name().to_string());
                        
                        // Phân giải Field lấy Damage (AttackType) bên trong DamagePropertyEvent
                        let dmg_event_class = method.arg(2).class();
                        let iter = std::ptr::null();
                        loop {
                            let field = unsafe { crate::kreide::il2cpp::api::il2cpp_class_get_fields(dmg_event_class, &iter) };
                            if field.0 == 0 { break; }
                            let type_name = unsafe { crate::kreide::il2cpp::api::il2cpp_type_get_name(crate::kreide::il2cpp::api::il2cpp_field_get_type(field)) };
                            let type_str = unsafe { crate::kreide::il2cpp::util::cstr_to_str(type_name) };
                            
                            // Field chứa AttackType (DOODKEMMAPK)
                            if type_str == "RPG.GameCore.AttackType" {
                                register("DamageEvent_AttackTypeField", field.name().to_string());
                            }
                        }
                        
                        found_damage = true;
                        break;
                    }
                }
            }
        }
    }

    // =========================================================================
    // 4. Phân giải TeamFormationItem (GPFCKFCIKNI)
    // Dấu vết: Field _TeamFormationDatas của RPG.GameCore.TeamFormationComponent
    // =========================================================================
    let team_formation_class = get_cached_class("RPG.GameCore.TeamFormationComponent")?;
    let iter = std::ptr::null();
    loop {
        let field = unsafe { crate::kreide::il2cpp::api::il2cpp_class_get_fields(team_formation_class, &iter) };
        if field.0 == 0 { break; }
        if field.name() == "_TeamFormationDatas" {
            let type_name = unsafe { crate::kreide::il2cpp::api::il2cpp_type_get_name(crate::kreide::il2cpp::api::il2cpp_field_get_type(field)) };
            let type_str = unsafe { crate::kreide::il2cpp::util::cstr_to_str(type_name) };
            // type_str sẽ có dạng "System.Collections.Generic.List`1<GPFCKFCIKNI>"
            if let Some(start) = type_str.find('<') {
                if let Some(end) = type_str.find('>') {
                    let item_class_name = &type_str[start + 1..end];
                    register("TeamFormationItem", item_class_name.to_string());
                }
            }
            break;
        }
    }

	// =========================================================================
    // 5. Phân giải BPIOPFEPAEG (Tham số của DirectDamageHP / DirectChangeHP)
    // Dấu vết: Tham số thứ 4 của hàm DirectDamageHP trong TurnBasedAbilityComponent
    // =========================================================================
    let tba_class = get_cached_class("RPG.GameCore.TurnBasedAbilityComponent")?;
    let direct_dmg_method = tba_class.methods().into_iter()
        .find(|m| m.name() == "DirectDamageHP" && m.args_cnt() == 6)
        .context("Không tìm thấy hàm DirectDamageHP")?;
    
    // Arg 0: fModifyValue, Arg 1: multiplier, Arg 2: eStrength, Arg 3: pParams (BPIOPFEPAEG)
    let param_params_name = direct_dmg_method.arg(3).class().name().to_string();
    register("DirectDamageParams", param_params_name);


    // =========================================================================
    // 6. Phân giải Fields của FHPFLNJLDHP (InsertSkillContext)
    // Dấu vết: Quét các field bên trong nó, check type để lấy tên field bị obfuscate
    // =========================================================================
    let insert_skill_ctx_name = get_dynamic_name("InsertSkillContext");
    let insert_skill_ctx_class = get_cached_class(&insert_skill_ctx_name)?;
    
    let iter = std::ptr::null();
    loop {
        let field = unsafe { crate::kreide::il2cpp::api::il2cpp_class_get_fields(insert_skill_ctx_class, &iter) };
        if field.0 == 0 { break; }
        
        let type_name = unsafe { crate::kreide::il2cpp::util::cstr_to_str(
            crate::kreide::il2cpp::api::il2cpp_type_get_name(crate::kreide::il2cpp::api::il2cpp_field_get_type(field))
        )};
        
        // Tìm Field có kiểu SkillCharacterComponent (Trong dump của bạn là MMALDILNGNJ)
        if type_name == "RPG.GameCore.SkillCharacterComponent" {
            register("InsertSkill_SkillChar_Field", field.name().to_string());
        }
        // Tìm Field có kiểu TurnBasedAbilityComponent (Trong dump của bạn là HHOKFHMEFFF)
        else if type_name == "RPG.GameCore.TurnBasedAbilityComponent" {
            register("InsertSkill_TurnBased_Field", field.name().to_string());
        }
        // Tìm Field chứa OOMAKCLAFOH. Vì OOMAKCLAFOH là struct bị mã hóa, ta lấy type name của field nào KHÔNG thuộc GameCore.
        else if !type_name.starts_with("System") && !type_name.starts_with("RPG") && !type_name.contains("[]") {
            // Đây chính là OOMAKCLAFOH (hoặc struct tương đương)
            register("InsertSkill_ComboData_Class", type_name.to_string());
            register("InsertSkill_ComboData_Field", field.name().to_string());
        }
    }

    // =========================================================================
    // 7. Phân giải Field 'String' bên trong OOMAKCLAFOH (Chứa tên skill Combo)
    // Dấu vết: Tìm field kiểu 'string' duy nhất ở vị trí Offset 0x8
    // =========================================================================
    let combo_data_class_name = get_dynamic_name("InsertSkill_ComboData_Class");
    if combo_data_class_name != "InsertSkill_ComboData_Class" { // Nếu tìm thấy
        let combo_data_class = get_cached_class(&combo_data_class_name)?;
        let iter = std::ptr::null();
        loop {
            let field = unsafe { crate::kreide::il2cpp::api::il2cpp_class_get_fields(combo_data_class, &iter) };
            if field.0 == 0 { break; }
            
            let type_name = unsafe { crate::kreide::il2cpp::util::cstr_to_str(
                crate::kreide::il2cpp::api::il2cpp_type_get_name(crate::kreide::il2cpp::api::il2cpp_field_get_type(field))
            )};
            
            // Tìm field kiểu "string" (Trong IL2CPP type name là System.String)
            if type_name == "System.String" {
                register("ComboData_String_Field", field.name().to_string());
                break;
            }
        }
    }

    log::info!("[Resolver] Đã hoàn tất phân giải IL2CPP!");
    Ok(())
}

pub fn resolve_dynamic_damage(instance_ptr: usize, expected_damage: Option<f64>) -> Option<f64> {
    if let Some(offset) = *DAMAGE_FIELD_OFFSET.lock().unwrap() {
        let fixpoint = unsafe { &*((instance_ptr + offset) as *const crate::kreide::types::RPG_GameCore_FixPoint) };
        return Some(crate::kreide::helpers::fixpoint_to_raw(fixpoint));
    }

    if let Some(delta) = expected_damage {
        let class_name = get_dynamic_name("DamagePropertyEvent");
        let Ok(class) = get_cached_class(&class_name) else { return None; };

        let mut current_matches = HashSet::new();
        let iter = std::ptr::null();
        
        log::debug!("[DamageResolver] Đang tìm offset cho Damage xấp xỉ: {}", delta);

        loop {
            let field = unsafe { crate::kreide::il2cpp::api::il2cpp_class_get_fields(class, &iter) };
            if field.0 == 0 { break; }

            let type_name = unsafe { crate::kreide::il2cpp::util::cstr_to_str(crate::kreide::il2cpp::api::il2cpp_type_get_name(crate::kreide::il2cpp::api::il2cpp_field_get_type(field))) };
            
            if type_name == "RPG.GameCore.FixPoint" {
                let offset = unsafe { crate::kreide::il2cpp::api::il2cpp_field_get_offset(field) } as usize;
                let fixpoint = unsafe { &*((instance_ptr + offset) as *const crate::kreide::types::RPG_GameCore_FixPoint) };
                let val = crate::kreide::helpers::fixpoint_to_raw(fixpoint);

                if (val - delta).abs() <= 2.0 {
                    log::debug!("[DamageResolver] Khớp tiềm năng tại Offset 0x{:X} | Giá trị: {}", offset, val);
                    current_matches.insert(offset);
                }
            }
        }

        let mut candidates_guard = DAMAGE_CANDIDATES.lock().unwrap();

        if let Some(candidates) = candidates_guard.as_mut() {
            let previous_count = candidates.len();
            candidates.retain(|o| current_matches.contains(o));
            let current_count = candidates.len();

            log::debug!("[DamageResolver] Số lượng offset còn lại sau khi giao nhau: {}", current_count);

            // Logic chốt số:
            if current_count == 1 {
                let final_offset = *candidates.iter().next().unwrap();
                *DAMAGE_FIELD_OFFSET.lock().unwrap() = Some(final_offset);
                log::info!("[DamageResolver] THÀNH CÔNG! Chốt Offset của Damage là: 0x{:X}", final_offset);
                
                let fixpoint = unsafe { &*((instance_ptr + final_offset) as *const crate::kreide::types::RPG_GameCore_FixPoint) };
                return Some(crate::kreide::helpers::fixpoint_to_raw(fixpoint));
            } else if current_count > 1 {
                // Nếu số lượng offset không giảm đi sau đòn đánh này, tăng biến đếm
                if current_count == previous_count {
                    let mut stagnant_count = INTERSECTION_STAGNANT_COUNT.lock().unwrap();
                    *stagnant_count += 1;
                    
                    // Nếu trùng nhau 3 lần liên tiếp (chắc chắn là các field clone nhau), CHỐT LUÔN ĐẠI 1 CÁI
                    if *stagnant_count >= 3 {
                        let mut sorted_offsets: Vec<_> = candidates.iter().cloned().collect();
                        sorted_offsets.sort(); // Lấy offset thấp nhất hoặc cao nhất cho ổn định (vd 0x370 thay vì 0x718)
                        let final_offset = sorted_offsets[0]; // Bạn có thể chọn sorted_offsets.last() nếu thích 0x718 hơn
                        
                        *DAMAGE_FIELD_OFFSET.lock().unwrap() = Some(final_offset);
                        log::info!("[DamageResolver] THÀNH CÔNG (Heuristic)! Sau 3 lần không đổi, chốt đại Offset: 0x{:X} từ {} ứng viên", final_offset, current_count);
                        
                        let fixpoint = unsafe { &*((instance_ptr + final_offset) as *const crate::kreide::types::RPG_GameCore_FixPoint) };
                        return Some(crate::kreide::helpers::fixpoint_to_raw(fixpoint));
                    }
                } else {
                    // Nếu số lượng có giảm đi, reset biến đếm
                    *INTERSECTION_STAGNANT_COUNT.lock().unwrap() = 0;
                }
            } else if candidates.is_empty() {
                log::warn!("[DamageResolver] Giao điểm rỗng! Thử lại ở đòn sau...");
                *candidates_guard = None;
                *INTERSECTION_STAGNANT_COUNT.lock().unwrap() = 0; // Reset
            }
        } else if !current_matches.is_empty() {
            if current_matches.len() == 1 {
                let final_offset = *current_matches.iter().next().unwrap();
                *DAMAGE_FIELD_OFFSET.lock().unwrap() = Some(final_offset);
                log::info!("[DamageResolver] THÀNH CÔNG! Chốt Offset của Damage (ngay lần 1) là: 0x{:X}", final_offset);
                
                let fixpoint = unsafe { &*((instance_ptr + final_offset) as *const crate::kreide::types::RPG_GameCore_FixPoint) };
                return Some(crate::kreide::helpers::fixpoint_to_raw(fixpoint));
            } else {
                log::debug!("[DamageResolver] Lần quét đầu tiên tìm thấy {} offsets tiềm năng. Chờ đòn sau để phân loại.", current_matches.len());
                *candidates_guard = Some(current_matches);
            }
        }
    }
    None
}