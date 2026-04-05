use std::collections::HashMap;
use std::fmt;

use crate::kreide::types::RPG_GameCore_AbilityProperty;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Avatar {
    pub id: u32,
    pub name: String,
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Enemy {
    pub id: u32,
    pub uid: u32,
    pub name: String,
    pub base_stats: BattleStats,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleEntity {
    pub entity: Entity,
    pub properties: BattleStats
}


#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct BattleStats {
    pub properties: HashMap<String, f64>,
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entity {
    pub uid: u32,
    pub team: Team
}

#[derive(PartialEq, Clone, Debug, Deserialize, Serialize)]
pub enum Team {
    Player,
    Enemy
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Property {
    pub value: f64,
    pub kind: String,
}

impl BattleStats {
    pub fn set_property(&mut self, property: Property) {
        self.properties.insert(property.kind, property.value);
    }

    pub fn set_value<S: Into<String>>(&mut self, kind: S, value: f64) {
        self.properties.insert(kind.into(), value);
    }

    pub fn get_value(&self, kind: &str) -> Option<f64> {
        self.properties.get(kind).copied()
    }

    pub fn current_hp(&self) -> f64 {
        let key = RPG_GameCore_AbilityProperty::CurrentHP.to_string();
        self.get_value(&key).unwrap_or_default()
    }

    pub fn av(&self) -> f64 {
        let key = RPG_GameCore_AbilityProperty::ActionDelay.to_string();
        self.get_value(&key).unwrap_or_default()
    }
	
	pub fn current_stance(&self) -> f64 {
		self.get_value("CurrentStance").unwrap_or_default()
	}
	
	pub fn max_stance(&self) -> f64 {
		self.get_value("MaxStance").unwrap_or_default()
	}
	
    pub fn max_hp(&self) -> f64 {
        let key = RPG_GameCore_AbilityProperty::MaxHP.to_string();
        self.get_value(&key).unwrap_or_default()
    }

    pub fn level(&self) -> u32 {
        self.get_value("Level").unwrap_or_default() as u32
    }
}

impl fmt::Display for Avatar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Skill {
    pub name: String,
    #[serde(rename = "type")]
    pub skill_type: String,
    pub skill_config_id: isize
}

impl fmt::Display for Skill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.skill_type, self.name)
    }
}


#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct TurnInfo {
    pub action_value: f64,
    pub cycle: u32,
    pub wave: u32,
    pub avatars_turn_damage: Vec<f64>,
    pub total_damage: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CharacterSummary {
    pub total_damage: f64,
    pub dpav: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleSummary {
    pub team_name: String,
    pub lineup: Vec<String>,
    pub lineup_details: Vec<Avatar>,
    pub timestamp: String,
    pub total_damage: f64,
    pub total_av: f64,
    pub total_dpav: f64,
    pub characters: HashMap<String, CharacterSummary>,
	pub turn_history: Vec<TurnInfo>,
    pub av_history: Vec<TurnInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Substat {
    pub key: String,
    pub value: f64,
    pub count: u32,
    pub step: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelicMainStat {
    pub stat: String,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelicRolls {
    pub high: u32,
    pub mid: u32,
    pub low: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelicSubstat {
    pub stat: String,
    pub value: f64,
    pub rolls: RelicRolls,
    #[serde(rename = "addedRolls")]
    pub added_rolls: u32,
	#[serde(skip)]
    pub raw_count: u32,
    #[serde(skip)]
    pub raw_step: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relic {
    pub part: String,
    #[serde(skip)]
    pub set_id: String,
    #[serde(rename = "set")]
    pub set: String,
    pub enhance: u32,
    pub grade: u32,
    pub main: RelicMainStat,
    pub substats: Vec<RelicSubstat>,
	#[serde(skip)]
    pub reroll_substats: Option<Vec<Substat>>,
    #[serde(skip)]
    pub preview_substats: Option<Vec<Substat>>,
    #[serde(rename = "equippedBy")]
    pub equipped_by: String,
    pub verified: bool,
    pub id: String,
    #[serde(rename = "ageIndex")]
    pub age_index: u32,
    #[serde(rename = "initialRolls")]
    pub initial_rolls: u32,
    #[serde(skip)]
    pub lock: bool,
    #[serde(skip)]
    pub discard: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReliquaryRelic {
    pub set_id: String,
    pub name: String,
    pub slot: String,
    pub rarity: u32,
    pub level: u32,
    pub mainstat: String,
    pub substats: Vec<Substat>,
	#[serde(skip_serializing_if = "Option::is_none")]
    pub reroll_substats: Option<Vec<Substat>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_substats: Option<Vec<Substat>>,
    pub location: String,
    pub lock: bool,
    pub discard: bool,
    pub _uid: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightCone {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub promotion: u32,
    pub rank: u32,
    pub equipped_by: String,
    pub lock: bool,
    pub uid: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReliquaryLightCone {
    pub id: String,
    pub name: String,
    pub level: u32,
    pub ascension: u32,
    pub superimposition: u32,
    pub location: String,
    pub lock: bool,
    pub _uid: String,
}

impl From<&LightCone> for ReliquaryLightCone {
    fn from(lc: &LightCone) -> Self {
        ReliquaryLightCone {
            id: lc.id.clone(),
            name: lc.name.clone(),
            level: lc.level,
            ascension: lc.promotion,
            superimposition: lc.rank,
            location: lc.equipped_by.clone(),
            lock: lc.lock,
            _uid: lc.uid.clone(),
        }
    }
}

impl From<&Relic> for ReliquaryRelic {
    fn from(relic: &Relic) -> Self {
        let substats = relic
            .substats
            .iter()
            .map(|substat| {
                let key = substat.stat.replace('%', "_");

                Substat {
					key,
					value: substat.value,
					count: substat.raw_count, 
					step: substat.raw_step,   
				}
            })
            .collect();

        ReliquaryRelic {
            set_id: relic.set_id.clone(),
            name: relic.set.clone(),
            slot: relic.part.clone(),
            rarity: relic.grade,
            level: relic.enhance,
            mainstat: if let Some(base) = relic.main.stat.strip_suffix('%') {
                base.to_string()
            } else {
                relic.main.stat.clone()
            },
            substats,
			reroll_substats: relic.reroll_substats.clone(),
            preview_substats: relic.preview_substats.clone(),
            location: relic.equipped_by.clone(),
            lock: relic.lock,
            discard: relic.discard,
            _uid: relic.id.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelicConfigDumpEntry {
    pub id: u32,
    pub set_id: u32,
    pub rarity: i32,
    #[serde(rename = "type")]
    pub relic_type: String,
    pub max_level: i32,
    pub main_affix_id: u32,
    pub sub_affix_id: u32,
    pub icon: String,
    pub name: String,
}

// --- FRIBBELS STRUCTURES ---
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FribbelsMetadata {
    pub uid: u32,
    pub trailblazer: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FribbelsArchive {
    pub source: String,
    pub build: String,
    pub version: u32,
    pub metadata: FribbelsMetadata,
    pub light_cones: Vec<ReliquaryLightCone>,
    pub relics: Vec<ReliquaryRelic>,
    pub characters: Vec<FribbelsCharacter>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FribbelsSkills {
    pub basic: u32,
    pub skill: u32,
    pub ult: u32,
    pub talent: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elation: Option<u32>,
}

// Cập nhật Traces (Xóa các field cũ, giữ đúng chuẩn mới)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FribbelsTraces {
    pub ability_1: bool,
    pub ability_2: bool,
    pub ability_3: bool,
    pub stat_1: bool,
    pub stat_2: bool,
    pub stat_3: bool,
    pub stat_4: bool,
    pub stat_5: bool,
    pub stat_6: bool,
    pub stat_7: bool,
    pub stat_8: bool,
    pub stat_9: bool,
    pub stat_10: bool,
    pub special: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FribbelsCharacter {
    pub id: String,
    pub name: String,
    pub path: String,
    pub level: u32,
    pub ascension: u32,
    pub eidolon: u32,
    pub skills: FribbelsSkills,
    pub traces: FribbelsTraces,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memosprite: Option<FribbelsMemosprite>,
    pub ability_version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FribbelsMemosprite {
    pub skill: u32,
    pub talent: u32,
}

impl FribbelsMemosprite {
    pub fn if_present(self) -> Option<Self> {
        if self.skill == 0 && self.talent == 0 { None } else { Some(self) }
    }
}