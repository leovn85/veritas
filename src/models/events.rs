use crate::kreide::types::RPG_GameCore_AttackType;
use super::misc::{Avatar, Enemy, Entity, Skill, /*Property, */Team};

pub enum Event {
    OnBattleBegin(OnBattleBeginEvent),
    OnSetBattleLineup(OnSetLineupEvent),
    OnDamage(OnDamageEvent),
    OnTurnBegin(OnTurnBeginEvent),
    OnTurnEnd,
    OnUseSkill(OnUseSkillEvent),
    OnBattleEnd,
    OnUpdateWave(OnUpdateWaveEvent),
    OnUpdateCycle(OnUpdateCycleEvent),
    OnPropertyChange(OnPropertyChangeEvent),
	OnSyncAvatarStats(OnSyncAvatarStatsEvent),
    OnEntityDefeated(OnEntityDefeatedEvent),
    OnUpdateTeamFormation(OnUpdateTeamFormationEvent),
    OnInitializeEnemy(OnInitializeEnemyEvent),
    OnResetActionValue(OnResetActionValueEvent),
}

pub struct OnResetActionValueEvent {
    pub action_value: f64,
}

pub struct OnBattleBeginEvent {
    pub max_waves: u32,
    pub max_cycles: u32,
    pub stage_id: u32
}

pub struct OnUpdateWaveEvent {
    pub wave: u32,
}

pub struct OnUpdateCycleEvent {
    pub cycle: u32,
}

pub struct OnTurnBeginEvent {
    pub action_value: f64,
    pub turn_owner: Option<Entity>
}

pub struct OnUseSkillEvent {
    pub avatar: Entity,
    pub skill: Skill
}

pub struct OnSetLineupEvent {
    pub avatars: Vec<Avatar>,
}

pub struct OnDamageEvent {
    pub attacker: Entity,
    pub damage: f64,
    pub overkill_damage: f64,
    //pub r#type: String,
	pub r#type: RPG_GameCore_AttackType,
}

pub struct OnEntityDefeatedEvent {
    pub killer: Entity,
    pub entity_defeated: Entity
}

pub struct OnPropertyChangeEvent {
    pub entity: Entity,
    pub property_type: &'static str, // Con trỏ tĩnh Copy trên Stack, không cấp phát bộ nhớ!
    pub value: f64,
}

pub struct OnSyncAvatarStatsEvent {
    pub avatar_id: u32,
    pub stats: Vec<(&'static str, f64)>, // Mảng phẳng gồm các &'static str, cực nhanh!
}

pub struct OnUpdateTeamFormationEvent {
    pub entities: Vec<Entity>,
    pub team: Team
}

pub struct OnInitializeEnemyEvent {
    pub enemy: Enemy
}

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}
