use std::fmt;
use std::collections::HashMap;

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
    pub base_stats: Stats,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleEntity {
    pub entity: Entity,
    pub battle_stats: BattleStats
}


#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct BattleStats {
    pub hp: f64,
    pub attack: f64,
    pub defense: f64,
    pub speed: f64,
    pub av: f64
}


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stats {
    pub level: u32,
    pub hp: f64
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
pub enum Stat {
    MaxHP(f64),
    BaseHP(f64),
    HPAddedRatio(f64),
    HPDelta(f64),
    HPConvert(f64),
    DirtyHPDelta(f64),
    DirtyHPRatio(f64),
    RallyHP(f64),
    NegativeHP(f64),
    HP(f64),
    MaxSP(f64),
    CurrentSP(f64),
    MaxSpecialSP(f64),
    CurrentSpecialSP(f64),
    AdditionalBP(f64),
    Attack(f64),
    BaseAttack(f64),
    AttackAddedRatio(f64),
    AttackDelta(f64),
    AttackConvert(f64),
    Defense(f64),
    BaseDefence(f64),
    DefenceAddedRatio(f64),
    DefenceDelta(f64),
    DefenceConvert(f64),
    DefenceOverride(f64),
    Level(f64),
    Promotion(f64),
    Rank(f64),
    Speed(f64),
    BaseSpeed(f64),
    SpeedAddedRatio(f64),
    SpeedDelta(f64),
    SpeedConvert(f64),
    SpeedOverride(f64),
    AV(f64),
    ActionDelayAddedRatio(f64),
    ActionDelayAddAttenuation(f64),
    MaxStance(f64),
    CurrentStance(f64),
    Level_AllDamageAddedRatio(f64),
    AllDamageTypeAddedRatio(f64),
    AllDamageReduce(f64),
    DotDamageAddedRatio(f64),
    FatigueRatio(f64),
    CriticalChance(f64),
    CriticalChanceBase(f64),
    CriticalChanceConvert(f64),
    CriticalDamage(f64),
    CriticalDamageBase(f64),
    CriticalDamageConvert(f64),
    CriticalResistance(f64),
    PhysicalAddedRatio(f64),
    FireAddedRatio(f64),
    IceAddedRatio(f64),
    ThunderAddedRatio(f64),
    QuantumAddedRatio(f64),
    ImaginaryAddedRatio(f64),
    WindAddedRatio(f64),
    PhysicalResistance(f64),
    FireResistance(f64),
    IceResistance(f64),
    ThunderResistance(f64),
    QuantumResistance(f64),
    ImaginaryResistance(f64),
    WindResistance(f64),
    PhysicalResistanceBase(f64),
    FireResistanceBase(f64),
    IceResistanceBase(f64),
    ThunderResistanceBase(f64),
    QuantumResistanceBase(f64),
    ImaginaryResistanceBase(f64),
    WindResistanceBase(f64),
    PhysicalResistanceDelta(f64),
    FireResistanceDelta(f64),
    IceResistanceDelta(f64),
    ThunderResistanceDelta(f64),
    QuantumResistanceDelta(f64),
    ImaginaryResistanceDelta(f64),
    WindResistanceDelta(f64),
    AllDamageTypeResistance(f64),
    PhysicalPenetrate(f64),
    FirePenetrate(f64),
    IcePenetrate(f64),
    ThunderPenetrate(f64),
    QuantumPenetrate(f64),
    ImaginaryPenetrate(f64),
    WindPenetrate(f64),
    AllDamageTypePenetrate(f64),
    PhysicalTakenRatio(f64),
    FireTakenRatio(f64),
    IceTakenRatio(f64),
    ThunderTakenRatio(f64),
    QuantumTakenRatio(f64),
    ImaginaryTakenRatio(f64),
    WindTakenRatio(f64),
    AllDamageTypeTakenRatio(f64),
    Monster_DamageTakenRatio(f64),
    PhysicalAbsorb(f64),
    FireAbsorb(f64),
    IceAbsorb(f64),
    ThunderAbsorb(f64),
    QuantumAbsorb(f64),
    ImaginaryAbsorb(f64),
    WindAbsorb(f64),
    MinimumFatigueRatio(f64),
    ForceStanceBreakRatio(f64),
    StanceBreakAddedRatio(f64),
    StanceBreakResistance(f64),
    StanceBreakTakenRatio(f64),
    PhysicalStanceBreakTakenRatio(f64),
    FireStanceBreakTakenRatio(f64),
    IceStanceBreakTakenRatio(f64),
    ThunderStanceBreakTakenRatio(f64),
    WindStanceBreakTakenRatio(f64),
    QuantumStanceBreakTakenRatio(f64),
    ImaginaryStanceBreakTakenRatio(f64),
    StanceWeakAddedRatio(f64),
    StanceDefaultAddedRatio(f64),
    HealRatio(f64),
    HealRatioBase(f64),
    HealRatioConvert(f64),
    HealTakenRatio(f64),
    Shield(f64),
    MaxShield(f64),
    ShieldAddedRatio(f64),
    ShieldTakenRatio(f64),
    StatusProbability(f64),
    StatusProbabilityBase(f64),
    StatusProbabilityConvert(f64),
    StatusResistance(f64),
    StatusResistanceBase(f64),
    StatusResistanceConvert(f64),
    SPRatio(f64),
    SPRatioBase(f64),
    SPRatioConvert(f64),
    SPRatioOverride(f64),
    BreakDamageAddedRatio(f64),
    BreakDamageAddedRatioBase(f64),
    BreakDamageAddedRatioConvert(f64),
    BreakDamageExtraAddedRatio(f64),
    PhysicalStanceBreakResistance(f64),
    FireStanceBreakResistance(f64),
    IceStanceBreakResistance(f64),
    ThunderStanceBreakResistance(f64),
    WindStanceBreakResistance(f64),
    QuantumStanceBreakResistance(f64),
    ImaginaryStanceBreakResistance(f64),
    AggroBase(f64),
    AggroAddedRatio(f64),
    AggroDelta(f64),
    RelicValueExtraAdditionRatio(f64),
    EquipValueExtraAdditionRatio(f64),
    EquipExtraRank(f64),
    AvatarExtraRank(f64),
    Combo(f64),
    NormalBattleCount(f64),
    ExtraAttackAddedRatio1(f64),
    ExtraAttackAddedRatio2(f64),
    ExtraAttackAddedRatio3(f64),
    ExtraAttackAddedRatio4(f64),
    ExtraDefenceAddedRatio1(f64),
    ExtraDefenceAddedRatio2(f64),
    ExtraDefenceAddedRatio3(f64),
    ExtraDefenceAddedRatio4(f64),
    ExtraHPAddedRatio1(f64),
    ExtraHPAddedRatio2(f64),
    ExtraHPAddedRatio3(f64),
    ExtraHPAddedRatio4(f64),
    ExtraHealAddedRatio(f64),
    ExtraAllDamageTypeAddedRatio1(f64),
    ExtraAllDamageTypeAddedRatio2(f64),
    ExtraAllDamageTypeAddedRatio3(f64),
    ExtraAllDamageTypeAddedRatio4(f64),
    ExtraAllDamageReduce(f64),
    ExtraShieldAddedRatio(f64),
    ExtraSpeedAddedRatio1(f64),
    ExtraSpeedAddedRatio2(f64),
    ExtraSpeedAddedRatio3(f64),
    ExtraSpeedAddedRatio4(f64),
    ExtraLuckChance(f64),
    ExtraLuckDamage(f64),
    ExtraFrontPower(f64),
    ExtraFrontPowerBase(f64),
    ExtraFrontPowerAddedRatio1(f64),
    ExtraFrontPowerAddedRatio2(f64),
    ExtraBackPower(f64),
    ExtraBackPowerBase(f64),
    ExtraBackPowerAddedRatio1(f64),
    ExtraBackPowerAddedRatio2(f64),
    ExtraUltraDamageAddedRatio1(f64),
    ExtraSkillDamageAddedRatio1(f64),
    ExtraNormalDamageAddedRatio1(f64),
    ExtraInsertDamageAddedRatio1(f64),
    ExtraDOTDamageAddedRatio1(f64),
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
    pub skill_type: isize,
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
}