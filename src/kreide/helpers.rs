
use std::{collections::HashMap, ptr::null, sync::LazyLock, collections::BTreeMap};

use crate::{
    kreide::types::{
        RPG_Client_AvatarData, RPG_Client_CachedAssetLoader, RPG_Client_GlobalVars,
        RPG_Client_ModuleManager, RPG_Client_UIGameEntityUtils, RPG_GameCore_AttackType__Boxed,
        RPG_GameCore_AvatarExcelTable, RPG_GameCore_AvatarPropertyExcelTable, RPG_GameCore_AvatarPropertyType__Boxed, RPG_GameCore_MonsterDataComponent, RPG_GameCore_MonsterRowData, 
        RPG_GameCore_ServantDataComponent, UnityEngine_Graphics, UnityEngine_ImageConversion,
        UnityEngine_Rect, UnityEngine_RenderTexture, UnityEngine_Sprite, UnityEngine_Texture2D, RPG_GameCore_RelicConfigExcelTable, RPG_GameCore_RelicSetConfigExcelTable, RPG_GameCore_RelicBaseTypeExcelTable,  RPG_GameCore_AvatarSkillTreeExcelTable, RPG_GameCore_AvatarBaseType, RPG_GameCore_AvatarRow, RPG_GameCore_RelicConfigRow
    },
    models::misc::{Avatar, Skill, FribbelsCharacter, FribbelsSkills, FribbelsTraces, FribbelsMemosprite},
};
use anyhow::{Context, Result, anyhow};
use function_name::named;
use il2cpp_runtime::{
    Il2CppObject, System_RuntimeType, get_cached_class,
    types::{Il2CppString, System_Enum, System_Int32__Boxed, System_Type},
};
use il2cpp_runtime::api::{il2cpp_class_get_fields, il2cpp_field_get_offset};

use super::types::{
    RPG_Client_TextID, RPG_Client_TextmapStatic,
    RPG_GameCore_BattleInstance, RPG_GameCore_FixPoint, RPG_GameCore_GameEntity,
    RPG_GameCore_SkillData,
};

pub fn sanitize_entity_name<S: AsRef<str>>(name: S) -> String {
    let name = name.as_ref();
    if !name.contains("<ub>") && !name.contains("</ub>") {
        return name.to_string();
    }

    name.replace("<ub>", "").replace("</ub>", "")
}

pub fn get_textmap_content(hash: &RPG_Client_TextID) -> Result<String> {
    Ok(unsafe { RPG_Client_TextmapStatic::get_text(hash, null()) }.map(|s| s.to_string())?)
}

#[named]
pub fn get_module_manager() -> Result<RPG_Client_ModuleManager> {
    log::debug!(function_name!());
    Ok(RPG_Client_GlobalVars::s_ModuleManager()?)
}

#[named]
pub fn get_avatar_data_from_id(avatar_id: u32) -> Result<RPG_Client_AvatarData> {
    log::debug!(function_name!());
    let s_module_manager = get_module_manager()?;
    let avatar_module = s_module_manager.AvatarModule()?;
    Ok(unsafe { avatar_module.get_avatar(avatar_id)? })
}

#[named]
pub unsafe fn get_avatar_from_id(avatar_id: u32) -> Result<Avatar> {
    log::debug!(function_name!());

    let avatar_data = get_avatar_data_from_id(avatar_id)
        .context(format!("AvatarData with id {avatar_id} was null"))?;

    let avatar_name = unsafe { avatar_data.AvatarName() }
        .map(|name| name.to_string())
        .unwrap_or_default();

    let avatar_name = if avatar_name.is_empty() {
        let data = unsafe { RPG_GameCore_AvatarExcelTable::GetData(avatar_id)? };
        get_textmap_content(&*data.AvatarName()?)?
    } else {
        avatar_name
    };

    Ok(Avatar {
        id: avatar_id,
        name: sanitize_entity_name(avatar_name),
    })
}

#[named]
pub unsafe fn get_skill_from_skilldata(skill_data: RPG_GameCore_SkillData) -> Result<Skill> {
    log::debug!(function_name!());

    if skill_data.0.is_null() {
        return Err(anyhow!("SkillData was null"));
    }

    let row_data = skill_data.RowData()?;

    let text_id = unsafe { row_data.get_SkillName()? };

    let skill_type = unsafe {
        let boxed = RPG_GameCore_AttackType__Boxed(System_Enum::to_object_from_int(
            get_type_handle("RPG.GameCore.AttackType")?,
            row_data.get_AttackType()? as i32,
        )?);
        System_Enum::get_name(get_type_handle("RPG.GameCore.AttackType")?, boxed.0)?.to_string()
    };

    Ok(Skill {
        name: get_textmap_content(&text_id)?,
        skill_type,
        skill_config_id: isize::try_from(*skill_data.SkillConfigID()?)?,
    })
}

#[named]
pub unsafe fn get_avatar_from_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());

    if entity.0.is_null() {
        return Err(anyhow!("Avatar entity was null"));
    }

    let id = unsafe { RPG_Client_UIGameEntityUtils::get_avatar_id(entity) }
        .context("Failed to get AvatarID from GameEntity")?;

    let avatar_data =
        get_avatar_data_from_id(id).context(format!("AvatarData with id {id} was null"))?;

    let name = unsafe { avatar_data.AvatarName() }
        .map(|name| name.to_string())
        .unwrap_or_default();

    Ok(Avatar {
        id,
        name: sanitize_entity_name(name),
    })
}

#[named]
pub unsafe fn get_avatar_from_servant_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());

    if entity.0.is_null() {
        return Err(anyhow!("Servant Entity was null"));
    }

    let battle_instance = entity
        ._OwnerWorldRef()?
        ._BattleInstanceRef_k__BackingField()?;

    let entity_manager = battle_instance._GameWorld()?._EntityManager()?;
    let avatar_entity = unsafe { entity_manager.get_entity_summoner(entity)? };
    unsafe { get_avatar_from_entity(avatar_entity) }
}

#[named]
pub unsafe fn get_monster_from_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());
    let monster_data_comp = RPG_GameCore_MonsterDataComponent(
        unsafe {
            entity.get_component(System_RuntimeType::from_name(
                "RPG.GameCore.MonsterDataComponent",
            )?)?
        }
        .0,
    );

    if monster_data_comp.0.is_null() {
        return Err(anyhow!("entity does not have MonsterDataComponent!"));
    }

    let monster_name = monster_data_comp._MonsterRowData()?._Row()?.MonsterName()?;

    let monster_id = unsafe { monster_data_comp.get_monster_id()? };

    Ok(Avatar {
        id: monster_id,
        name: sanitize_entity_name(get_textmap_content(&*monster_name)?),
    })
}

#[named]
pub unsafe fn get_servant_from_entity(entity: RPG_GameCore_GameEntity) -> Result<Avatar> {
    log::debug!(function_name!());
    let servant_data_comp = RPG_GameCore_ServantDataComponent(
        unsafe {
            entity.get_component(System_RuntimeType::from_name(
                "RPG.GameCore.ServantDataComponent",
            )?)?
        }
        .0,
    );

    if servant_data_comp.0.is_null() {
        return Err(anyhow!("entity does not have ServantDataComponent!"));
    }

    let servant_row = servant_data_comp._ServantRowData()?._Row()?;

    Ok(Avatar {
        id: u32::try_from(*servant_row.ServantID()?)?,
        name: sanitize_entity_name(get_textmap_content(&*servant_row.ServantName()?)?),
    })
}

// #[named]
// pub unsafe fn get_entity_modifiers(entity: RPG_GameCore_GameEntity) -> Result<Vec<Value>> {
//     log::debug!(function_name!());
//     let ability_comp = RPG_GameCore_AbilityComponent(
//         entity
//             .get_component(System_RuntimeType::from_name("RPG.GameCore.AbilityComponent")?)?
//             .0,
//     );

//     if ability_comp.0.is_null() {
//         return Err(anyhow!("entity does not have AbilityComponent!"));
//     }

//     let modifier_list = List(ability_comp._ModifierList()?.0);
//     let modifier_list_array = modifier_list.to_vec::<RPG_GameCore_TurnBasedModifierInstance>();

//     Ok(modifier_list_array
//         .iter()
//         .filter_map(|obj| {
//             let status_config_key = obj.get_key_for_status_config().ok()?;

//             let status_row =
//                 RPG_GameCore_StatusExcelTable::get_by_modifier_name(status_config_key).ok()?;

//             Some(if status_row.is_null() {
//                 json!({
//                     "key": status_config_key.as_str(),
//                 })
//             } else {
//                 json!({
//                     "key": status_config_key.as_str(),
//                     "desc": get_textmap_content(&status_row.StatusDesc().ok()?),
//                     "name": get_textmap_content(&status_row.StatusName().ok()?),
//                 })
//             })
//         })
//         .collect::<Vec<_>>())
// }

// pub unsafe fn get_entity_ability_properties(
//     entity: RPG_GameCore_GameEntity,
// ) -> Result<HashMap<String, f64>> {
//     let ability_comp = RPG_GameCore_TurnBasedAbilityComponent(
//         unsafe {
//             entity.get_component(System_RuntimeType::from_name(
//                 "RPG.GameCore.TurnBasedAbilityComponent",
//             )?)?
//         }
//         .0,
//     );

//     if ability_comp.0.is_null() {
//         return Err(anyhow!("entity does not have TurnBasedAbilityComponent!"));
//     }

//     Ok((0..=193)
//         .filter_map(|i| {
//             let property_enum =
//                 unsafe { std::mem::transmute::<i32, RPG_GameCore_AbilityProperty>(i) };
//             let value = fixpoint_to_raw(&unsafe { ability_comp.get_property(property_enum).ok()? });

//             (value != 0.0).then_some((format!("{property_enum:?}"), value))
//         })
//         .collect::<HashMap<String, f64>>())
// }

#[named]
pub unsafe fn get_monster_from_runtime_id(
    id: u32,
    battle_instance: RPG_GameCore_BattleInstance,
) -> Result<Avatar> {
    log::debug!(function_name!());
    unsafe {
        get_monster_from_entity(
            battle_instance
                ._GameWorld()?
                ._EntityManager()?
                .get_entity_by_runtime_id(id)?,
        )
    }
}

#[named]
pub fn fixpoint_to_raw(fixpoint: &RPG_GameCore_FixPoint) -> f64 {
    log::debug!(function_name!());
    static FLOAT_CONVERSION_CONSTANT: LazyLock<f64> = LazyLock::new(|| 1f64 / 2f64.powf(32f64));
    let raw_value = fixpoint.m_rawValue;
    let hi = ((raw_value as u64 & 0xFFFFFFFF00000000) >> 32) as u32;
    let lo = (raw_value as u64 & 0x00000000FFFFFFFF) as u32;
    hi as f64 + lo as f64 * *FLOAT_CONVERSION_CONSTANT
}

pub fn is_obfuscated_name<S: AsRef<str>>(name: S) -> bool {
    let name = name.as_ref();
    name.len() == 11 && name.chars().all(|c| c.is_ascii_uppercase())
}

pub fn get_type_handle<S: AsRef<str>>(type_name: S) -> Result<System_Type> {
    let type_name = type_name.as_ref();
    let runtime_type = System_RuntimeType::from_name(type_name)?;
    let ty = runtime_type.get_il2cpp_type();
    Ok(unsafe { System_Type::get_type_from_handle(ty)? })
}

/// Extract render texture formats for texture-to-PNG conversion
unsafe fn get_render_texture_formats() -> Result<(i32, i32)> {
    unsafe {
        let default_format = {
            let value = System_Int32__Boxed(System_Enum::parse(
                get_type_handle("UnityEngine.RenderTextureFormat")?,
                Il2CppString::new("Default")?,
            )?);
            (*value).0
        };

        let rw_format = {
            let value = System_Int32__Boxed(System_Enum::parse(
                get_type_handle("UnityEngine.RenderTextureReadWrite")?,
                Il2CppString::new("Linear")?,
            )?);
            (*value).0
        };

        Ok((default_format, rw_format))
    }
}

/// Common texture rendering pipeline: texture → render target → readable texture → PNG bytes
unsafe fn render_texture_to_png_bytes(tex: UnityEngine_Texture2D) -> Result<Vec<u8>> {
    unsafe {
        let (default_format, rw_format) = get_render_texture_formats()?;

        let render_tex = UnityEngine_RenderTexture::GetTemporary(
            tex.as_base().get_width()?,
            tex.as_base().get_height()?,
            0,
            default_format,
            rw_format,
        )?;
        UnityEngine_Graphics::Blit(tex, render_tex)?;
        let prev = UnityEngine_RenderTexture::GetActive()?;
        UnityEngine_RenderTexture::set_active(render_tex)?;

        use il2cpp_runtime::api::il2cpp_object_new;
        let readable_tex = UnityEngine_Texture2D(il2cpp_object_new(get_cached_class(
            UnityEngine_Texture2D::ffi_name(),
        )?));

        readable_tex.new(tex.as_base().get_width()?, tex.as_base().get_height()?)?;
        readable_tex.read_pixels(
            UnityEngine_Rect {
                x: 0.,
                y: 0.,
                width: render_tex.get_width()? as f32,
                height: render_tex.get_height()? as f32,
            },
            0,
            0,
        )?;
        readable_tex.apply()?;
        UnityEngine_RenderTexture::set_active(prev)?;
        UnityEngine_RenderTexture::ReleaseTemporary(render_tex)?;

        let array = UnityEngine_ImageConversion::EncodeToPNG(readable_tex)?;
        Ok(array.to_vec::<u8>())
    }
}

pub fn get_monster_png_bytes(monster_row_data: &RPG_GameCore_MonsterRowData) -> Result<Vec<u8>> {
    unsafe {
        //let monster_row = RPG_GameCore_MonsterTemplateExcelTable::GetData(monster_id)?;		
		let icon_path = monster_row_data.get_RoundIconPath()?; 
		
		if icon_path.0.is_null() {
			return Err(anyhow::anyhow!("Monster IconPath is null"));
		}
		
        let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;

        let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(
            //monster_row.RoundIconPath()?,
			icon_path,
            type_handle,
            false,
        )?;
        let sprite = UnityEngine_Sprite(sprite.0);
        let tex = sprite.get_texture()?;

        render_texture_to_png_bytes(tex)
    }
}

pub fn get_avatar_png_bytes(avatar_id: u32) -> Result<Vec<u8>> {
    unsafe {
        let avatar_row = RPG_GameCore_AvatarExcelTable::GetData(avatar_id)?;
        log::info!(
            "Support Avatar: {}, Icon Path: {}",
            avatar_id,
            avatar_row.AvatarSideIconPath()?.to_string()
        );

        let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;

        let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(
            avatar_row.AvatarSideIconPath()?,
            type_handle,
            false,
        )?;
        let sprite = UnityEngine_Sprite(sprite.0);
        let tex = sprite.get_texture()?;

        render_texture_to_png_bytes(tex)
    }
}

pub fn get_property_icon_png_bytes(property_name: &str) -> Result<Vec<u8>> {
    unsafe {
        let property_type = RPG_GameCore_AvatarPropertyType__Boxed(System_Enum::parse(
            get_type_handle("RPG.GameCore.AvatarPropertyType")?,
            Il2CppString::new(property_name)?,
        )?);
        
        let row = RPG_GameCore_AvatarPropertyExcelTable::GetData(*property_type)?;
        let icon_path = row.IconPath()?;

        let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;
        
        let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(
            icon_path,
            type_handle,
            false,
        )?;
        let sprite = UnityEngine_Sprite(sprite.0);
        let tex = sprite.get_texture()?;

        render_texture_to_png_bytes(tex)
    }
}

pub fn dump_avatar_png_bytes(avatar_id: u32, png_bytes: &[u8]) -> Result<()> {
    use std::fs;
    let out_dir = std::env::current_dir()?.join("avatar_png_dumps");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{}.png", avatar_id));
    fs::write(&out_path, png_bytes)?;

    log::info!("Saved avatar PNG dump: {}", out_path.display());
    Ok(())
}

// pub fn get_monster_png_bytes(monster_row_data: &RPG_GameCore_MonsterRowData) -> Result<Vec<u8>> {
	// unsafe {
		// let icon_path = monster_row_data.get_RoundIconPath()?; 
		
		// if icon_path.0.is_null() {
			// return Err(anyhow::anyhow!("Monster IconPath is null"));
		// }

		// let type_handle = get_type_handle(UnityEngine_Sprite::ffi_name())?;

		// let sprite = RPG_Client_CachedAssetLoader::SyncLoadAsset(icon_path, type_handle, false)?;
		// let sprite = UnityEngine_Sprite(sprite.0);
		// let tex = sprite.get_texture()?;

		// let default_format = {
			// let value = System_Int32__Boxed(System_Enum::parse(
				// get_type_handle("UnityEngine.RenderTextureFormat")?,
				// Il2CppString::new("Default")?,
			// )?);
			// (*value).0
		// };

		// let rw_format = {
			// let value = System_Int32__Boxed(System_Enum::parse(
				// get_type_handle("UnityEngine.RenderTextureReadWrite")?,
				// Il2CppString::new("Linear")?,
			// )?);
			// (*value).0
		// };

		// let render_tex = UnityEngine_RenderTexture::GetTemporary(
			// tex.as_base().get_width()?,
			// tex.as_base().get_height()?,
			// 0,
			// default_format,
			// rw_format,
		// )?;
		// UnityEngine_Graphics::Blit(tex, render_tex)?;
		// let prev = UnityEngine_RenderTexture::GetActive()?;
		// UnityEngine_RenderTexture::set_active(render_tex)?;
		// use il2cpp_runtime::api::il2cpp_object_new;
		// let readable_tex = UnityEngine_Texture2D(il2cpp_object_new(get_cached_class(
			// UnityEngine_Texture2D::ffi_name(),
		// )?));

		// readable_tex.new(tex.as_base().get_width()?, tex.as_base().get_height()?)?;
		// readable_tex.read_pixels(
			// UnityEngine_Rect {
				// x: 0.,
				// y: 0.,
				// width: render_tex.get_width()? as f32,
				// height: render_tex.get_height()? as f32,
			// },
			// 0,
			// 0,
		// )?;
		// readable_tex.apply()?;
		// UnityEngine_RenderTexture::set_active(prev)?;
		// UnityEngine_RenderTexture::ReleaseTemporary(render_tex)?;

		// let array = UnityEngine_ImageConversion::EncodeToPNG(readable_tex)?;
		// let buffer = array.to_vec::<u8>();
		// Ok(buffer)
	// }
// }

pub unsafe fn dump_characters_to_json() -> anyhow::Result<()> {
    //let mut characters = HashMap::new();
	let mut characters = BTreeMap::new();

    let dict_ptr = unsafe { RPG_GameCore_AvatarExcelTable::get_dataDict()? };
    let rows = unsafe { extract_rows_from_dict_ram(dict_ptr).unwrap_or_default() };

    for row_ptr in rows {
        let avatar_row = RPG_GameCore_AvatarRow(row_ptr as _);
        let avatar_id = u32::from(*avatar_row.AvatarID()?);
        
        if let Ok(name_id) = avatar_row.AvatarName() {
            if (*name_id).hash != 0 {
                // XỬ LÝ RIÊNG CHO MAIN CHARACTER (ID >= 8000)
                if avatar_id >= 8000 {
                    // Thử lấy tên thật, bọc trong try_seh để chống crash
                    let safe_fetch = microseh::try_seh(|| {
                        get_textmap_content(&*name_id)
                    });

                    let mut got_real_name = false;

                    // Nếu lấy thành công và không bị crash
                    if let Ok(Ok(name_str)) = safe_fetch {
                        let clean_name = sanitize_entity_name(name_str);
                        if !clean_name.is_empty() {
                            characters.insert(avatar_id, clean_name);
                            got_real_name = true;
                        }
                    }

                    // Nếu crash (SEH) hoặc lỗi, Fallback về tên cứng
                    if !got_real_name {
                        log::debug!("Failed to get real name for MC {}, using fallback.", avatar_id);
                        let mc_name = match avatar_id {
                            8001 => "Destruction Male MC",
                            8002 => "Destruction Female MC",
                            8003 => "Preservation Male MC",
                            8004 => "Preservation Female MC",
                            8005 => "Harmony Male MC",
                            8006 => "Harmony Female MC",
                            8007 => "Remembrance Male MC",
                            8008 => "Remembrance Female MC",
                            8009 => "Elation Male MC",
                            8010 => "Elation Female MC",
                            9982 => "Aether Divide MC",
                            _ => "Unknown Trailblazer",
                        };
                        characters.insert(avatar_id, mc_name.to_string());
                    }
                } 
                // XỬ LÝ CHO NHÂN VẬT BÌNH THƯỜNG (ID < 8000)
                else {
                    if let Ok(name_str) = get_textmap_content(&*name_id) {
                        let clean_name = sanitize_entity_name(name_str);
                        if !clean_name.is_empty() {
                            characters.insert(avatar_id, clean_name);
                        }
                    }
                }
            }
        }
    }

    let json = serde_json::to_string_pretty(&characters)?;
    std::fs::write("characters.json", json)?;
    log::info!("Dumped {} characters dynamically!", characters.len());
    Ok(())
}

pub unsafe fn dump_relic_sets() -> anyhow::Result<()> {
    //let mut relic_sets = HashMap::new();
	let mut relic_sets = BTreeMap::new();
    // Chỉ quét các set ID hợp lệ hiện tại
    let set_ids = (101..=200).chain(301..=400);

    for id in set_ids {
        if let Ok(row) = unsafe { RPG_GameCore_RelicSetConfigExcelTable::GetData(id) } {
            if !row.0.is_null() {
                if let Ok(name_id) = row.SetName() {
                    if (*name_id).hash != 0 {
                        if let Ok(name) = get_textmap_content(&*name_id) {
                            relic_sets.insert(id, name);
                        }
                    }
                }
            }
        }
    }

    std::fs::write("relic_sets.json", serde_json::to_string_pretty(&relic_sets)?)?;
    log::info!("Dumped {} relic sets!", relic_sets.len());
    Ok(())
}

pub unsafe fn dump_relic_config() -> anyhow::Result<()> {
    //let mut relic_configs = HashMap::new();
	let mut relic_configs = BTreeMap::new();
	
    log::debug!("Getting dataDict...");
    let dict_ptr = unsafe { RPG_GameCore_RelicConfigExcelTable::get_dataDict()? };
    
    log::debug!("Extracting rows from RAM...");
    let rows = unsafe { extract_rows_from_dict_ram(dict_ptr).unwrap_or_default() };
    log::debug!("Found {} rows to process.", rows.len());
	
	let rows_len = rows.len();
    for (index, row_ptr) in rows.into_iter().enumerate() {
        log::debug!("--- Processing Row {}/{} (Ptr: {:p}) ---", index + 1, rows_len, row_ptr);
        
        let row = RPG_GameCore_RelicConfigRow(row_ptr as _);

        // Lấy ID đầu tiên để biết đang xử lý Relic nào
        log::debug!("Reading ID...");
        let id = (*row.ID()?).0;
        log::debug!("=> ID: {}", id);

        log::debug!("Reading SetID...");
        let set_id = (*row.SetID()?).0;
        log::debug!("=> SetID: {}", set_id);

        log::debug!("Reading Rarity...");
        let rarity_val = *row.Rarity()? as i32 + 1;
        log::debug!("=> Rarity: {}", rarity_val);

        log::debug!("Reading Type...");
        let type_enum_val = *row.Type()?;
        log::debug!("=> Type Enum Val: {}", type_enum_val as i32);

        log::debug!("Converting Type Enum to String...");
		let type_enum_obj = unsafe { RPG_GameCore_RelicBaseTypeExcelTable::GetData(type_enum_val)? };
		let type_str = unsafe { RPG_Client_TextmapStatic::get_text(&*type_enum_obj.BaseTypeText()?, std::ptr::null())?.to_string() };
        //let type_enum_obj = System_Int32__Boxed(System_Enum::to_object_from_int(type_handle, type_enum_val as i32)?);
        //let type_str = System_Enum::get_name(type_handle, type_enum_obj.0)?.to_string();
        log::debug!("=> Type String: {}", type_str);

        log::debug!("Reading MaxLevel...");
        let max_level = (*row.MaxLevel()?).0;
        log::debug!("=> MaxLevel: {}", max_level);

        log::debug!("Reading MainAffixGroup...");
        let main_affix_id = (*row.MainAffixGroup()?).0;
        log::debug!("=> MainAffixGroup: {}", main_affix_id);

        log::debug!("Reading SubAffixGroup...");
        let sub_affix_id = (*row.SubAffixGroup()?).0;
        log::debug!("=> SubAffixGroup: {}", sub_affix_id);

        // 1. LẤY TÊN BỘ DI VẬT (Set Name) - Bọc thép
        let mut set_name_str = format!("Set {}", set_id); // Fallback
        let safe_set = microseh::try_seh(|| unsafe { RPG_GameCore_RelicSetConfigExcelTable::GetData(set_id) });
        
        if let Ok(Ok(set_row)) = safe_set {
            if !set_row.0.is_null() {
                let safe_name_id = microseh::try_seh(|| set_row.SetName());
                if let Ok(Ok(name_id)) = safe_name_id {
                    let safe_text = microseh::try_seh(|| get_textmap_content(&*name_id));
                    if let Ok(Ok(name)) = safe_text {
                        set_name_str = sanitize_entity_name(name);
                    }
                }
            }
        }

        // 2. LẤY TÊN VỊ TRÍ (Slot Name) - Bọc thép
        let mut slot_name_str = type_str.clone(); // Fallback (e.g., "HEAD", "OBJECT")
        //let relic_type_enum = std::mem::transmute::<i32, RPG_GameCore_RelicSetType>(type_enum_val);
        
        let safe_type = microseh::try_seh(|| unsafe { RPG_GameCore_RelicBaseTypeExcelTable::GetData(type_enum_val) });
        
        if let Ok(Ok(type_row)) = safe_type {
            if !type_row.0.is_null() {
                // Bọc try_seh ở đây để chống lỗi crash Reflection (GetFields) mà bạn vừa gặp
                let safe_text_id = microseh::try_seh(|| type_row.BaseTypeText());
                if let Ok(Ok(text_id)) = safe_text_id {
                    let safe_text = microseh::try_seh(|| get_textmap_content(&*text_id));
                    if let Ok(Ok(name)) = safe_text {
                        slot_name_str = sanitize_entity_name(name);
                    }
                }
            }
        }

        // 3. GHÉP TÊN VÀ BỎ ICON
        let final_name = format!("{} - {}", set_name_str, slot_name_str);
        let icon = "****".to_string(); // Bỏ Icon

        relic_configs.insert(
            id.to_string(),
            crate::models::misc::RelicConfigDumpEntry {
                id, set_id, rarity: rarity_val, relic_type: type_str, max_level,
                main_affix_id, sub_affix_id, icon, name: final_name,
            }
        );
        log::debug!("Successfully processed ID: {}", id);
    }

    log::debug!("Serialization to JSON...");
    let json = serde_json::to_string_pretty(&relic_configs)?;
    std::fs::write("relic_config_dump.json", json)?;
    log::info!("Dumped {} relic configs dynamically!", relic_configs.len());
    Ok(())
}

pub unsafe fn extract_keys_from_dict_ram(dict_ptr: *mut std::ffi::c_void) -> anyhow::Result<Vec<u32>> {
    if dict_ptr.is_null() {
        return Ok(Vec::new());
    }

    // Lấy Class từ pointer của Object
    let class_ptr = unsafe { *(dict_ptr as *const *const std::ffi::c_void) };
    let dict_class = il2cpp_runtime::Il2CppClass(class_ptr);

    let mut count_offset = 0;
    let mut entries_offset = 0;

    // Tìm offset của field `_count` và `_entries` tự động để chống lệch Version
    let field_iter: *const std::ffi::c_void = std::ptr::null();
    loop {
        //use il2cpp_runtime::api::{il2cpp_class_get_fields, il2cpp_field_get_offset};
        let field = il2cpp_class_get_fields(dict_class, &field_iter);
        if field.0.is_null() { break; }
        
        let name = field.name();
        if name == "_count" || name == "count" {
            count_offset = il2cpp_field_get_offset(field) as usize;
        } else if name == "_entries" || name == "entries" {
            entries_offset = il2cpp_field_get_offset(field) as usize;
        }
    }

    if count_offset == 0 || entries_offset == 0 {
        return Err(anyhow::anyhow!("Không tìm thấy cấu trúc _count hoặc _entries của Dictionary"));
    }

    // Đọc số lượng cấp phát
    let count = unsafe { *(dict_ptr.add(count_offset) as *const i32) };
    if count <= 0 { return Ok(Vec::new()); }

    // Đọc con trỏ mảng _entries
    let entries_array_ptr = unsafe { *(dict_ptr.add(entries_offset) as *const *const u8) };
    if entries_array_ptr.is_null() { return Ok(Vec::new()); }

    let mut ids = Vec::new();
    
    // Header của Il2cppArray dài 0x20 bytes, bỏ qua phần này để vào vùng data
    let array_data_start = unsafe { entries_array_ptr.add(0x20) };

    for i in 0..count {
        // Mỗi Entry dài 0x18 (24 bytes) trên bản 64-bit
        let entry_ptr = unsafe { array_data_start.add((i as usize) * 0x18) };
        
        // Đọc hashCode ở offset 0x00. Nếu >= 0 nghĩa là phần tử này chưa bị xóa.
        let hash_code = unsafe { *(entry_ptr.add(0x00) as *const i32) };
        if hash_code >= 0 {
            // Đọc Key ở offset 0x08. (Cấu trúc: int hash, int next, uint key)
            let key = unsafe { *(entry_ptr.add(0x08) as *const u32) };
            ids.push(key);
        }
    }

    Ok(ids)
}

pub unsafe fn extract_rows_from_dict_ram(dict_ptr: *mut std::ffi::c_void) -> anyhow::Result<Vec<*mut std::ffi::c_void>> {
    if dict_ptr.is_null() {
        return Ok(Vec::new());
    }

    let class_ptr = unsafe { *(dict_ptr as *const *const std::ffi::c_void) };
    let dict_class = il2cpp_runtime::Il2CppClass(class_ptr);

    let mut count_offset = 0;
    let mut entries_offset = 0;

    let field_iter: *const std::ffi::c_void = std::ptr::null();
    loop {
        let field = il2cpp_runtime::api::il2cpp_class_get_fields(dict_class, &field_iter);
        if field.0.is_null() { break; }
        
        let name = field.name();
        if name == "_count" || name == "count" {
            count_offset = il2cpp_runtime::api::il2cpp_field_get_offset(field) as usize;
        } else if name == "_entries" || name == "entries" {
            entries_offset = il2cpp_runtime::api::il2cpp_field_get_offset(field) as usize;
        }
    }

    if count_offset == 0 || entries_offset == 0 {
        return Err(anyhow::anyhow!("Không tìm thấy cấu trúc _count hoặc _entries"));
    }

    let count = unsafe { *(dict_ptr.add(count_offset) as *const i32) };
    if count <= 0 { return Ok(Vec::new()); }

    let entries_array_ptr = unsafe { *(dict_ptr.add(entries_offset) as *const *const u8) };
    if entries_array_ptr.is_null() { return Ok(Vec::new()); }

    let mut rows = Vec::new();
    let array_data_start = unsafe { entries_array_ptr.add(0x20) };

    for i in 0..count {
        let entry_ptr = unsafe { array_data_start.add((i as usize) * 0x18) };
        let hash_code = unsafe { *(entry_ptr.add(0x00) as *const i32) };
        
        // Nếu hashCode >= 0, entry này có chứa dữ liệu hợp lệ
        if hash_code >= 0 {
            // Đọc TValue ở offset 0x10 (Con trỏ trỏ đến AvatarRow / RelicConfigRow)
            let row_ptr = unsafe { *(entry_ptr.add(0x10) as *const *mut std::ffi::c_void) };
            if !row_ptr.is_null() {
                rows.push(row_ptr);
            }
        }
    }

    Ok(rows)
}

pub unsafe fn dump_fribbels_characters() -> anyhow::Result<(Vec<FribbelsCharacter>, u32, String)> {
    //log::info!("[Character Dump] Starting Fribbels character dump sequence...");
    let domain = il2cpp_runtime::api::il2cpp_domain_get();
	il2cpp_runtime::api::il2cpp_thread_attach(domain);
    let mut characters_map: BTreeMap<u32, FribbelsCharacter> = BTreeMap::new();
    let mut player_uid: u32 = 0;
    let mut account_name = "Unknown".to_string();
    let mut trailblazer_gender = "Stelle".to_string();

    // Lấy offset một lần duy nhất ở ngoài vòng lặp để tối ưu hiệu suất
    //let anchor_offset = get_field_offset("RPG.Client.AvatarSkillTreeData", "_PointIDOfAnchorType");
    let levels_offset = unsafe { get_field_offset("RPG.Client.AvatarSkillTreeData", "SkillTreeLevels") };
	unsafe {
		let safe_dump = microseh::try_seh(|| {
			let module_manager = RPG_Client_GlobalVars::s_ModuleManager()?;
			
			// ==========================================
			// 1. LẤY UID & TÊN TỪ PLAYER MODULE
			// ==========================================
			//log::info!("[Character Dump] Accessing Player Module...");
			let safe_player_data = microseh::try_seh(|| {
				let player_module = module_manager.PlayerModule()?;
				if !player_module.0.is_null() {
					let player_data = player_module.get_PlayerData()?;
					if !player_data.0.is_null() {
						// Lấy UID
						if let Ok(uid) = player_data.get_UserID() {
							player_uid = uid;
							//log::info!("[Character Dump] Successfully retrieved UserID: {}", player_uid);
						} else {
							log::debug!("[Character Dump] Failed to read UserID from PlayerData.");
						}
						
						// Lấy Tên
						if let Ok(name_str) = player_data.get_NickName() {
							let clean_name = sanitize_entity_name(name_str.to_string());
							if !clean_name.is_empty() {
								account_name = clean_name;
								//log::info!("[Character Dump] Successfully retrieved NickName: {}", account_name);
							}
						} else {
							log::debug!("[Character Dump] Failed to read NickName from PlayerData.");
						}
					} else {
						log::debug!("[Character Dump] PlayerData instance is NULL.");
					}
				} else {
					log::debug!("[Character Dump] PlayerModule instance is NULL.");
				}
				Ok::<(), anyhow::Error>(())
			});

			if let Err(e) = safe_player_data {
				log::debug!("[Character Dump] SEH Exception caught while reading PlayerData: {:#?}", e);
			}

			// ==========================================
			// 2. LẤY DANH SÁCH NHÂN VẬT TỪ DICTIONARY
			// ==========================================
			//log::info!("[Character Dump] Accessing Avatar Module...");
			let avatar_module = module_manager.AvatarModule()?;
			let mut all_row_ptrs = Vec::new();
			
			if let Ok(all_avatars) = avatar_module.get_AllAvatars() {
				if !all_avatars.as_ptr().is_null() {
					if let Ok(rows) = extract_rows_from_dict_ram(all_avatars.as_ptr() as _) {
						//log::info!("[Character Dump] Extracted {} rows from AllAvatars.", rows.len());
						all_row_ptrs.extend(rows);
					}
				}
			}
			
			if let Ok(multi_path_avatars) = avatar_module.get_AllMultiPathAvatars() {
				if !multi_path_avatars.as_ptr().is_null() {
					if let Ok(rows) = extract_rows_from_dict_ram(multi_path_avatars.as_ptr() as _) {
						//log::info!("[Character Dump] Extracted {} rows from AllMultiPathAvatars.", rows.len());
						all_row_ptrs.extend(rows);
					}
				}
			}

			if all_row_ptrs.is_empty() {
				log::debug!("[Character Dump] No avatars found. Aborting character dump.");
				return Ok(()); 
			}
			
			//log::info!("[Character Dump] Processing {} raw avatar entries...", all_row_ptrs.len());

			// ==========================================
			// 3. DUYỆT VÀ XỬ LÝ TỪNG NHÂN VẬT
			// ==========================================
			for (_i, row_ptr) in all_row_ptrs.iter().enumerate() {
				let avatar_data = RPG_Client_AvatarData(*row_ptr as _);
				
				let base_id = match avatar_data.get_RealID() {
					Ok(id) => id,
					Err(_) => continue,
				};

				// Detect Stelle hay Caelus cực kỳ an toàn
				if base_id >= 8000 && base_id < 9000 {
					let gender = if base_id % 2 == 0 { "Stelle" } else { "Caelus" };
					trailblazer_gender = gender.to_string();
					//log::info!("[Character Dump] Detected Trailblazer (ID: {}). Identified gender: {}", base_id, trailblazer_gender);
				}

				let avatar_row = RPG_GameCore_AvatarExcelTable::GetData(base_id)?;
				if avatar_row.0.is_null() { continue; }

				let path_enum_val = *avatar_row.AvatarBaseType()?;
				let level = avatar_data.get_Level().unwrap_or(1);
				let promotion = avatar_data.get_Promotion().unwrap_or(0);
				let rank = avatar_data.get_Rank().unwrap_or(0);
				let enhanced_id = avatar_data.get_EnhancedID().unwrap_or(0);
				let ability_version = if enhanced_id > 0 && enhanced_id != base_id && base_id < 8000 { 1 } else { 0 };

				let path_str = match path_enum_val {
					RPG_GameCore_AvatarBaseType::Warrior => "Destruction",
					RPG_GameCore_AvatarBaseType::Rogue => "Hunt",
					RPG_GameCore_AvatarBaseType::Mage => "Erudition",
					RPG_GameCore_AvatarBaseType::Shaman => "Harmony",
					RPG_GameCore_AvatarBaseType::Warlock => "Nihility",
					RPG_GameCore_AvatarBaseType::Knight => "Preservation",
					RPG_GameCore_AvatarBaseType::Priest => "Abundance",
					RPG_GameCore_AvatarBaseType::Memory => "Remembrance",
					RPG_GameCore_AvatarBaseType::Elation => "Elation",
					_ => "Unknown",
				};
				
				let mut name = format!("Avatar_{}", base_id);
				let name_safe_fetch = microseh::try_seh(|| {
					avatar_data.AvatarName().map(|s| s.to_string())
				});

				if let Ok(Ok(name_str)) = name_safe_fetch {
					let clean = sanitize_entity_name(name_str);
					if !clean.is_empty() {
						name = clean;
					}
				} else if base_id >= 8000 {
					name = format!("{} MC", path_str); 
				}
				
				// --- SKILL TREE (ĐỌC TRỰC TIẾP TỪ RAM ĐỂ TRÁNH SPAM LOG) ---
				let mut skills = FribbelsSkills { basic: 1, skill: 1, ult: 1, talent: 1, elation: None};
				let mut traces = FribbelsTraces {
					ability_1: false, ability_2: false, ability_3: false,
					stat_1: false, stat_2: false, stat_3: false, stat_4: false, stat_5: false,
					stat_6: false, stat_7: false, stat_8: false, stat_9: false, stat_10: false, special: false,
				};
				let mut memosprite = None;
				//log::debug!("[SkillTree Debug] [{}] ----------------------------------------", base_id);
				//log::debug!("[SkillTree Debug] [{}] Attempting to read SkillTreeData...", base_id);

				if let Ok(skill_tree_data) = avatar_data.SkillTreeData() {
					if skill_tree_data.0.is_null() {
						log::debug!("[SkillTree Debug] [{}] SkillTreeData pointer is NULL.", base_id);
					} else if levels_offset == 0 {
						log::error!("[SkillTree Debug] [{}] levels_offset is invalid (0x0)!", base_id);
					} else {
						let base_ptr = skill_tree_data.0 as *const u8;
						let level_ptr = *(base_ptr.add(levels_offset) as *const *mut std::ffi::c_void);

						//log::debug!("[SkillTree Debug] [{}] Read Pointer -> level_ptr: {:p}", base_id, level_ptr);

						if !level_ptr.is_null() {
							// 1. Lấy Dictionary chứa Level của các Node (PointID -> Level)
							let level_dict = extract_primitive_dict_ram(level_ptr).unwrap_or_default();
							//log::debug!("[SkillTree Debug] [{}] Extracted Dict -> level_dict size: {}", base_id, level_dict.len());
							
							let mut anchor_to_level: HashMap<u32, u32> = HashMap::new();

							// 2. Duyệt qua từng PointID mà nhân vật đang có
							for (&point_id, &level) in &level_dict {
								// Hỏi Excel Table xem PointID này là kỹ năng gì (Truyền Level = 1 để lấy base info)
								if let Ok(row) = RPG_GameCore_AvatarSkillTreeExcelTable::GetData(point_id, 1) {
									if !row.0.is_null() {
										if let Ok(anchor_box) = row.AnchorType() {
											let anchor_type = *anchor_box as u32;
											
											// Lưu vào map: AnchorType -> Level (Lấy level cao nhất nếu có trùng lặp)
											let current_max = anchor_to_level.get(&anchor_type).copied().unwrap_or(0);
											anchor_to_level.insert(anchor_type, std::cmp::max(current_max, level));
											
											//log::debug!("[SkillTree Debug] [{}] Mapped PointID: {} -> AnchorType: {}, Level: {}", base_id, point_id, anchor_type, level);
										} else {
											log::debug!("[SkillTree Debug] [{}] Failed to read AnchorType for PointID: {}", base_id, point_id);
										}
									} else {
										log::debug!("[SkillTree Debug] [{}] ExcelTable returned NULL row for PointID: {}", base_id, point_id);
									}
								} else {
									log::debug!("[SkillTree Debug] [{}] Failed to call GetData for PointID: {}", base_id, point_id);
								}
							}

							// 3. Gán dữ liệu vào struct Fribbels
							let get_lv = |anchor_type: u32| -> u32 { 
								anchor_to_level.get(&anchor_type).copied().unwrap_or(0) 
							};

							skills.basic = std::cmp::max(1, get_lv(1));
							skills.skill = std::cmp::max(1, get_lv(2));
							skills.ult = std::cmp::max(1, get_lv(3));
							skills.talent = std::cmp::max(1, get_lv(4));

							// Gán Elation nếu có
							let elation_lv = get_lv(22);
							skills.elation = if elation_lv > 0 { Some(elation_lv) } else { None };

							// Gán Memosprite (Pet)
							memosprite = FribbelsMemosprite {
								skill: get_lv(19),
								talent: get_lv(20),
							}.if_present();

							traces.ability_1 = get_lv(6) > 0; 
							traces.ability_2 = get_lv(7) > 0; 
							traces.ability_3 = get_lv(8) > 0;

							traces.stat_1 = get_lv(9) > 0; 
							traces.stat_2 = get_lv(10) > 0; 
							traces.stat_3 = get_lv(11) > 0;
							traces.stat_4 = get_lv(12) > 0; 
							traces.stat_5 = get_lv(13) > 0; 
							traces.stat_6 = get_lv(14) > 0;
							traces.stat_7 = get_lv(15) > 0; 
							traces.stat_8 = get_lv(16) > 0; 
							traces.stat_9 = get_lv(17) > 0;
							traces.stat_10 = get_lv(18) > 0;

							traces.special = get_lv(21) > 0;

							//log::info!("[SkillTree Debug] [{}] Final Skills: B:{}, S:{}, U:{}, T:{}", base_id, skills.basic, skills.skill, skills.ult, skills.talent);
						} else {
							log::debug!("[SkillTree Debug] [{}] level_ptr is NULL.", base_id);
						}
					}
				} else {
					log::debug!("[SkillTree Debug] [{}] Failed to call avatar_data.SkillTreeData()", base_id);
				}

				characters_map.insert(base_id, FribbelsCharacter {
					id: base_id.to_string(),
					name,
					path: path_str.to_string(),
					level,
					ascension: promotion,
					eidolon: rank,
					skills,
					traces,
					memosprite,
					ability_version,
				});
			}
			
			Ok::<(), anyhow::Error>(())
		});
		
		if let Err(e) = safe_dump {
			log::error!("[Character Dump] CRITICAL SEH EXCEPTION in main dump loop: {:#?}", e);
		}
	}
    //log::info!("[Character Dump] Successfully processed {} unique characters.", characters_map.len());
    
    let trailblazer_meta = format!("{} ({})", account_name, trailblazer_gender);
    //log::info!("[Character Dump] Final Metadata -> UID: {}, Trailblazer: {}", player_uid, trailblazer_meta);

    Ok((characters_map.into_values().collect::<Vec<_>>(), player_uid, trailblazer_meta))
}

pub unsafe fn extract_primitive_dict_ram(dict_ptr: *mut std::ffi::c_void) -> anyhow::Result<HashMap<u32, u32>> {
    //log::info!("[DictReader] Starting to read primitive dictionary at ptr: {:p}", dict_ptr);

    if dict_ptr.is_null() {
        //log::warn!("[DictReader] Dictionary pointer is null. Aborting.");
        return Ok(HashMap::new());
    }

    let class_ptr = unsafe { *(dict_ptr as *const *const std::ffi::c_void) };
    let dict_class = il2cpp_runtime::Il2CppClass(class_ptr);
    //log::info!("[DictReader] Dictionary class name: {}", dict_class.name());

    let mut count_offset = 0;
    let mut entries_offset = 0;

    let field_iter: *const std::ffi::c_void = std::ptr::null();
    loop {
        let field = il2cpp_runtime::api::il2cpp_class_get_fields(dict_class, &field_iter);
        if field.0.is_null() { break; }
        
        let name = field.name();
        if name == "_count" || name == "count" {
            count_offset = il2cpp_runtime::api::il2cpp_field_get_offset(field) as usize;
        } else if name == "_entries" || name == "entries" {
            entries_offset = il2cpp_runtime::api::il2cpp_field_get_offset(field) as usize;
        }
    }

    if count_offset == 0 || entries_offset == 0 {
        //log::error!("[DictReader] Could not find '_count' or '_entries' field offsets.");
        return Err(anyhow::anyhow!("Không tìm thấy _count hoặc _entries"));
    }
    //log::info!("[DictReader] Found offsets: _count -> {:#x}, _entries -> {:#x}", count_offset, entries_offset);

    let count = unsafe { *(dict_ptr.add(count_offset) as *const i32) };
    //log::info!("[DictReader] Dictionary count: {}", count);
    if count <= 0 { return Ok(HashMap::new()); }

    let entries_array_ptr = unsafe { *(dict_ptr.add(entries_offset) as *const *const u8) };
    //log::info!("[DictReader] Entries array pointer: {:p}", entries_array_ptr);
    if entries_array_ptr.is_null() { 
        //log::warn!("[DictReader] Entries array pointer is null. Aborting.");
        return Ok(HashMap::new()); 
    }

    let mut map = HashMap::new();
    let array_data_start = unsafe { entries_array_ptr.add(0x20) };
    //log::info!("[DictReader] Array data starts at: {:p}", array_data_start);

    for i in 0..count {
        // Entry size cho <int, uint> hoặc <uint, uint> là 0x10 (16 bytes)
        // Cấu trúc: hashCode (4 bytes) | next (4 bytes) | key (4 bytes) | value (4 bytes)
        let entry_ptr = unsafe { array_data_start.add((i as usize) * 0x10) };
        let hash_code = unsafe { *(entry_ptr.add(0x00) as *const i32) };
        
        if hash_code >= 0 {
            let key = unsafe { *(entry_ptr.add(0x08) as *const u32) };
            let value = unsafe { *(entry_ptr.add(0x0C) as *const u32) };
            map.insert(key, value);
            // Log mỗi 10 entry để tránh spam log
            if i % 10 == 0 {
                //log::info!("[DictReader] Read entry {}/{}: hash={}, key={}, value={}", i, count, hash_code, key, value);
            }
        }
    }
    
    //log::info!("[DictReader] Finished reading. Found {} valid entries.", map.len());
    Ok(map)
}

unsafe fn get_field_offset(class_name: &str, field_name: &str) -> usize {
    if let Ok(class) = il2cpp_runtime::get_cached_class(class_name) {
        let field_iter: *const std::ffi::c_void = std::ptr::null();
        loop {
            let field = il2cpp_runtime::api::il2cpp_class_get_fields(class, &field_iter);
            if field.0.is_null() { break; }
            let name = unsafe { il2cpp_runtime::utils::cstr_to_str(il2cpp_runtime::api::il2cpp_field_get_name(field)) };
            if name == field_name {
                return il2cpp_runtime::api::il2cpp_field_get_offset(field) as usize;
            }
        }
    }
    0
}