use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

#[cfg(feature = "damage-debug")]
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{anyhow, Result};
use log::info;
use protocol::{DamageDetails, DamageModifierKind, DamageStatusContribution};
use retour::static_detour;

use crate::process::Process;

use super::read_process_value;

type AttackMultiplierFunc =
    unsafe extern "system" fn(*const usize, *const usize, f32, f32, f32, f32, *const usize) -> f32;

type DefenseStatusFunc =
    unsafe extern "system" fn(*const usize, *mut f32, *const usize, *const usize);

static_detour! {
    static AttackMultiplier: unsafe extern "system" fn(
        *const usize,
        *const usize,
        f32,
        f32,
        f32,
        f32,
        *const usize
    ) -> f32;

    static DefenseStatus: unsafe extern "system" fn(
        *const usize,
        *mut f32,
        *const usize,
        *const usize
    );
}

const ATTACK_MULTIPLIER_RVA: usize = 0xBD9630;
const DEFENSE_STATUS_RVA: usize = 0xBD9A40;
const ATTACK_MULTIPLIER_PREFIX: &[u8] = &[
    0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x56, 0x57, 0x53, 0x48, 0x81, 0xEC, 0x28,
    0x01, 0x00, 0x00,
];
const DEFENSE_STATUS_PREFIX: &[u8] = &[
    0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x56, 0x57, 0x53, 0x48, 0x81, 0xEC, 0xD8,
    0x00, 0x00, 0x00,
];

#[cfg(feature = "damage-debug")]
const MAX_LOGGED_DAMAGE_EVENTS: usize = 200;
#[cfg(feature = "damage-debug")]
static LOGGED_DAMAGE_EVENTS: AtomicUsize = AtomicUsize::new(0);
static GAME_BASE: OnceLock<usize> = OnceLock::new();
static STATUS_LAYOUTS: OnceLock<Mutex<HashMap<usize, StatusLayout>>> = OnceLock::new();

const STATUS_VECTOR_BEGIN_OFFSET: usize = 0x18;
const STATUS_VECTOR_END_OFFSET: usize = 0x20;
const MAX_ACTIVE_STATUSES: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum StatusInterface {
    Attack,
    Defense,
    DamageLimit,
    BonusAttack,
    Amplify,
}

impl StatusInterface {
    fn from_rtti_name(name: &str) -> Option<Self> {
        match name {
            ".?AVIStatusAttackBuff@@" => Some(Self::Attack),
            ".?AVIStatusDeffenceBuff@@" => Some(Self::Defense),
            ".?AVIStatusDamageLimitBuff@@" => Some(Self::DamageLimit),
            ".?AVIStatusBonusAttackBuff@@" => Some(Self::BonusAttack),
            ".?AVIStatusAmplifyBuff@@" => Some(Self::Amplify),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
struct StatusLayout {
    class_name: String,
    interfaces: Vec<(StatusInterface, usize)>,
}

#[derive(Clone, Debug)]
struct StatusContributionProbe {
    class_name: String,
    interface: StatusInterface,
    category: i32,
    value: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct RawDamageFields {
    damage: i32,
    candidate_d8: f32,
    flags: u64,
    candidate_154: f32,
    action_id: u32,
    candidate_2b8: i32,
    damage_cap: i32,
    uncapped_damage: f32,
    elemental_multiplier: f32,
}

#[derive(Clone, Debug, Default)]
struct AttackMultiplierProbe {
    status: usize,
    target: usize,
    factor_x2: f32,
    factor_x3: f32,
    lower_bound: f32,
    reduction_floor: f32,
    context: usize,
    result: f32,
    statuses: Vec<StatusContributionProbe>,
}

#[derive(Clone, Debug, Default)]
struct DefenseStatusProbe {
    status: usize,
    context: usize,
    buckets: [f32; 7],
    statuses: Vec<StatusContributionProbe>,
}

#[derive(Clone, Debug)]
struct DamageProbe {
    address: usize,
    before: RawDamageFields,
    attacks: Vec<AttackMultiplierProbe>,
    defenses: Vec<DefenseStatusProbe>,
}

thread_local! {
    static DAMAGE_PROBES: RefCell<Vec<DamageProbe>> = const { RefCell::new(Vec::new()) };
}

pub fn setup(process: &Process) -> Result<()> {
    let _ = GAME_BASE.set(process.base_address);
    let attack_address = process.base_address + ATTACK_MULTIPLIER_RVA;
    let defense_address = process.base_address + DEFENSE_STATUS_RVA;

    if !matches_prefix(attack_address, ATTACK_MULTIPLIER_PREFIX) {
        return Err(anyhow!("attack multiplier function prefix mismatch"));
    }
    if !matches_prefix(defense_address, DEFENSE_STATUS_PREFIX) {
        return Err(anyhow!("defense status function prefix mismatch"));
    }

    unsafe {
        let attack: AttackMultiplierFunc = std::mem::transmute(attack_address);
        AttackMultiplier.initialize(
            attack,
            |status, target, factor_x2, factor_x3, lower_bound, reduction_floor, context| {
                attack_multiplier_hook(
                    status,
                    target,
                    factor_x2,
                    factor_x3,
                    lower_bound,
                    reduction_floor,
                    context,
                )
            },
        )?;
        AttackMultiplier.enable()?;

        let defense: DefenseStatusFunc = std::mem::transmute(defense_address);
        DefenseStatus.initialize(defense, |status, output, target, context| {
            defense_status_hook(status, output, target, context)
        })?;
        DefenseStatus.enable()?;
    }

    info!("Damage detail functions: attack={attack_address:#x}, defense={defense_address:#x}");

    Ok(())
}

fn matches_prefix(address: usize, expected: &[u8]) -> bool {
    let actual = unsafe { std::slice::from_raw_parts(address as *const u8, expected.len()) };
    actual == expected
}

pub fn begin_damage(instance: *const usize) {
    DAMAGE_PROBES.with(|probes| {
        probes.borrow_mut().push(DamageProbe {
            address: instance as usize,
            before: read_raw_fields(instance),
            attacks: Vec::new(),
            defenses: Vec::new(),
        });
    });
}

pub fn finish_damage(instance: *const usize, _original_value: usize) -> Option<DamageDetails> {
    let probe = DAMAGE_PROBES.with(|probes| probes.borrow_mut().pop());
    let Some(probe) = probe else {
        return None;
    };

    let after = read_raw_fields(instance);

    #[cfg(feature = "damage-debug")]
    {
        let event_index = LOGGED_DAMAGE_EVENTS.fetch_add(1, Ordering::Relaxed);
        if event_index < MAX_LOGGED_DAMAGE_EVENTS {
            info!(
                "Damage detail #{event_index}: ptr={:#x}, result={}, before={:?}, after={:?}, attack_calls={:?}, defense_calls={:?}",
                probe.address,
                _original_value,
                probe.before,
                after,
                probe.attacks,
                probe.defenses,
            );
        }
    }

    Some(build_damage_details(&probe, after))
}

fn build_damage_details(probe: &DamageProbe, after: RawDamageFields) -> DamageDetails {
    // The game invokes both aggregation functions twice for the same hit. The
    // first completed call contains the same resolved values as the second, so
    // consume one snapshot to avoid double-counting every status.
    let attack_call = probe.attacks.first();
    let defense_call = probe.defenses.first();

    let mut statuses = Vec::new();
    if let Some(call) = attack_call {
        statuses.extend(call.statuses.iter().cloned());
    }
    if let Some(call) = defense_call {
        statuses.extend(call.statuses.iter().cloned());
    }

    let attack_multiplier = 1.0
        + statuses
            .iter()
            .filter(|status| status.interface == StatusInterface::Attack)
            .map(|status| status.value)
            .sum::<f32>();
    let amplify_multiplier = 1.0
        + statuses
            .iter()
            .filter(|status| status.interface == StatusInterface::Amplify)
            .map(|status| status.value)
            .sum::<f32>();
    let damage_limit_multiplier = 1.0
        + statuses
            .iter()
            .filter(|status| status.interface == StatusInterface::DamageLimit)
            .map(|status| status.value)
            .sum::<f32>();

    let defense_multiplier = defense_call
        .map(|call| effective_defense_multiplier(call.buckets))
        .unwrap_or(1.0);
    let elemental_multiplier = valid_multiplier(after.elemental_multiplier).unwrap_or(1.0);
    let supplementary_multiplier = valid_supplementary_ratio(after.candidate_154)
        .map(|ratio| 1.0 + ratio)
        .unwrap_or(1.0);
    let formula_multiplier = calculate_formula_multiplier(
        elemental_multiplier,
        amplify_multiplier,
        defense_multiplier,
        attack_multiplier,
        supplementary_multiplier,
    );

    DamageDetails {
        elemental_multiplier,
        amplify_multiplier,
        defense_multiplier,
        attack_multiplier,
        supplementary_multiplier,
        formula_multiplier,
        attack_rate: finite_or_default(after.candidate_d8, 0.0),
        uncapped_damage: finite_or_default(after.uncapped_damage, 0.0),
        damage_cap: after.damage_cap,
        damage_limit_multiplier,
        statuses: statuses
            .into_iter()
            .map(|status| DamageStatusContribution {
                status_name: status.class_name,
                kind: match status.interface {
                    StatusInterface::Attack => DamageModifierKind::Attack,
                    StatusInterface::Defense => DamageModifierKind::Defense,
                    StatusInterface::DamageLimit => DamageModifierKind::DamageLimit,
                    StatusInterface::BonusAttack => DamageModifierKind::BonusAttack,
                    StatusInterface::Amplify => DamageModifierKind::Amplify,
                },
                category: status.category,
                value: status.value,
            })
            .collect(),
    }
}

fn calculate_formula_multiplier(
    elemental: f32,
    amplify: f32,
    defense: f32,
    attack: f32,
    supplementary: f32,
) -> f32 {
    (elemental * amplify + (defense * attack - 1.0) / 2.0) * supplementary
}

fn effective_defense_multiplier(buckets: [f32; 7]) -> f32 {
    let multiplier = (1.0 + buckets[0])
        * (1.0 - buckets[1])
        * (1.0 - buckets[2])
        * (1.0 + buckets[3] + buckets[4] - buckets[5] - buckets[6]);
    valid_multiplier(multiplier).unwrap_or(1.0)
}

fn valid_multiplier(value: f32) -> Option<f32> {
    (value.is_finite() && value > 0.0 && value <= 20.0).then_some(value)
}

fn valid_supplementary_ratio(value: f32) -> Option<f32> {
    (value.is_finite() && (0.0..=10.0).contains(&value)).then_some(value)
}

fn finite_or_default(value: f32, default: f32) -> f32 {
    value.is_finite().then_some(value).unwrap_or(default)
}

unsafe extern "system" fn attack_multiplier_hook(
    status: *const usize,
    target: *const usize,
    factor_x2: f32,
    factor_x3: f32,
    lower_bound: f32,
    reduction_floor: f32,
    context: *const usize,
) -> f32 {
    let result = AttackMultiplier.call(
        status,
        target,
        factor_x2,
        factor_x3,
        lower_bound,
        reduction_floor,
        context,
    );
    let statuses = collect_status_contributions(status, context, None, None);

    DAMAGE_PROBES.with(|probes| {
        if let Some(probe) = probes.borrow_mut().last_mut() {
            probe.attacks.push(AttackMultiplierProbe {
                status: status as usize,
                target: target as usize,
                factor_x2,
                factor_x3,
                lower_bound,
                reduction_floor,
                context: context as usize,
                result,
                statuses,
            });
        }
    });

    result
}

unsafe extern "system" fn defense_status_hook(
    status: *const usize,
    output: *mut f32,
    target: *const usize,
    context: *const usize,
) {
    DefenseStatus.call(status, output, target, context);

    if output.is_null() {
        return;
    }

    let buckets = std::array::from_fn(|index| output.add(index).read_unaligned());
    let statuses = collect_status_contributions(
        status,
        context,
        Some(target),
        Some(StatusInterface::Defense),
    );
    DAMAGE_PROBES.with(|probes| {
        if let Some(probe) = probes.borrow_mut().last_mut() {
            probe.defenses.push(DefenseStatusProbe {
                status: status as usize,
                context: context as usize,
                buckets,
                statuses,
            });
        }
    });
}

fn collect_status_contributions(
    status_manager: *const usize,
    context: *const usize,
    defense_target: Option<*const usize>,
    only_interface: Option<StatusInterface>,
) -> Vec<StatusContributionProbe> {
    if status_manager.is_null() {
        return Vec::new();
    }

    let manager_address = status_manager as usize;
    let Some(begin) =
        read_process_value::<usize>((manager_address + STATUS_VECTOR_BEGIN_OFFSET) as *const usize)
    else {
        return Vec::new();
    };
    let Some(end) =
        read_process_value::<usize>((manager_address + STATUS_VECTOR_END_OFFSET) as *const usize)
    else {
        return Vec::new();
    };

    if begin == 0 || end < begin || (end - begin) % std::mem::size_of::<usize>() != 0 {
        return Vec::new();
    }

    let count = ((end - begin) / std::mem::size_of::<usize>()).min(MAX_ACTIVE_STATUSES);
    let mut contributions = Vec::new();
    let mut visited = HashSet::new();

    for index in 0..count {
        let Some(status_address) = read_process_value::<usize>(
            (begin + index * std::mem::size_of::<usize>()) as *const usize,
        ) else {
            continue;
        };
        if status_address == 0 {
            continue;
        }
        if !visited.insert(status_address) {
            continue;
        }

        let Some(primary_vtable) = read_process_value::<usize>(status_address as *const usize)
        else {
            continue;
        };
        let Some(layout) = status_layout(primary_vtable) else {
            continue;
        };

        for (interface, offset) in &layout.interfaces {
            if only_interface.is_some_and(|expected| expected != *interface) {
                continue;
            }

            let interface_address = status_address + offset;
            let Some(interface_vtable) =
                read_process_value::<usize>(interface_address as *const usize)
            else {
                continue;
            };

            if *interface == StatusInterface::Defense {
                if let Some(target) = defense_target {
                    let Some(predicate_address) =
                        read_process_value::<usize>((interface_vtable + 0x18) as *const usize)
                    else {
                        continue;
                    };
                    type DefensePredicate =
                        unsafe extern "system" fn(*const usize, *const usize, *const usize) -> bool;
                    let predicate: DefensePredicate =
                        unsafe { std::mem::transmute(predicate_address) };
                    if !unsafe { predicate(interface_address as *const usize, target, context) } {
                        continue;
                    }
                }
            }

            let Some(getter_address) =
                read_process_value::<usize>((interface_vtable + 8) as *const usize)
            else {
                continue;
            };
            type StatusValueGetter = unsafe extern "system" fn(*const usize, *const usize) -> f32;
            let getter: StatusValueGetter = unsafe { std::mem::transmute(getter_address) };
            let value = unsafe { getter(interface_address as *const usize, context) };
            if !value.is_finite() || value.abs() > 100.0 {
                continue;
            }

            let category =
                read_process_value::<i32>((interface_vtable - 0x0C) as *const i32).unwrap_or(-1);
            contributions.push(StatusContributionProbe {
                class_name: layout.class_name.clone(),
                interface: *interface,
                category,
                value,
            });
        }
    }

    contributions
}

fn status_layout(primary_vtable: usize) -> Option<StatusLayout> {
    if let Some(layout) = STATUS_LAYOUTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()?
        .get(&primary_vtable)
        .cloned()
    {
        return Some(layout);
    }

    let layout = parse_status_layout(primary_vtable)?;
    STATUS_LAYOUTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()?
        .insert(primary_vtable, layout.clone());
    Some(layout)
}

fn parse_status_layout(primary_vtable: usize) -> Option<StatusLayout> {
    let game_base = *GAME_BASE.get()?;
    let col_address = read_process_value::<usize>((primary_vtable - 8) as *const usize)?;
    let signature = read_process_value::<u32>(col_address as *const u32)?;
    if signature != 1 {
        return None;
    }

    let hierarchy_rva = read_process_value::<u32>((col_address + 0x10) as *const u32)? as usize;
    let hierarchy = game_base + hierarchy_rva;
    let base_count = read_process_value::<u32>((hierarchy + 8) as *const u32)? as usize;
    if base_count == 0 || base_count > 32 {
        return None;
    }
    let base_array_rva = read_process_value::<u32>((hierarchy + 0x0C) as *const u32)? as usize;
    let base_array = game_base + base_array_rva;

    let mut layout = StatusLayout::default();
    for index in 0..base_count {
        let descriptor_rva = read_process_value::<u32>(
            (base_array + index * std::mem::size_of::<u32>()) as *const u32,
        )? as usize;
        let descriptor = game_base + descriptor_rva;
        let type_descriptor_rva = read_process_value::<u32>(descriptor as *const u32)? as usize;
        let name = read_rtti_name(game_base + type_descriptor_rva + 0x10)?;
        let member_offset = read_process_value::<i32>((descriptor + 8) as *const i32)?;

        if index == 0 {
            layout.class_name = name
                .trim_start_matches(".?AV")
                .trim_end_matches("@@")
                .to_string();
        }
        if let Some(interface) = StatusInterface::from_rtti_name(&name) {
            if (0..=0x400).contains(&member_offset) {
                layout.interfaces.push((interface, member_offset as usize));
            }
        }
    }

    Some(layout)
}

fn read_rtti_name(address: usize) -> Option<String> {
    let bytes = read_process_value::<[u8; 128]>(address as *const [u8; 128])?;
    let length = bytes.iter().position(|byte| *byte == 0)?;
    String::from_utf8(bytes[..length].to_vec()).ok()
}

fn read_raw_fields(instance: *const usize) -> RawDamageFields {
    if instance.is_null() {
        return RawDamageFields::default();
    }

    unsafe {
        RawDamageFields {
            damage: instance.byte_add(0xD4).cast::<i32>().read_unaligned(),
            candidate_d8: instance.byte_add(0xD8).cast::<f32>().read_unaligned(),
            flags: instance.byte_add(0xE8).cast::<u64>().read_unaligned(),
            candidate_154: instance.byte_add(0x154).cast::<f32>().read_unaligned(),
            action_id: instance.byte_add(0x16C).cast::<u32>().read_unaligned(),
            candidate_2b8: instance.byte_add(0x2B8).cast::<i32>().read_unaligned(),
            damage_cap: instance.byte_add(0x2BC).cast::<i32>().read_unaligned(),
            uncapped_damage: instance.byte_add(0x2D4).cast::<f32>().read_unaligned(),
            elemental_multiplier: instance.byte_add(0x2D8).cast::<f32>().read_unaligned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{calculate_formula_multiplier, effective_defense_multiplier};

    #[test]
    fn defense_buckets_match_observed_additive_debuffs() {
        let mut buckets = [0.0; 7];
        buckets[4] = 0.65;
        assert!((effective_defense_multiplier(buckets) - 1.65).abs() < f32::EPSILON);
    }

    #[test]
    fn requested_damage_formula_is_applied_in_order() {
        let result = calculate_formula_multiplier(1.2, 1.15, 1.65, 1.2, 1.8);
        assert!((result - 3.366).abs() < 0.0001);
    }
}
