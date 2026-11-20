//! Chloe rides Sebastian; only landed swings advance the repeatable nine-hit roar.
use crate::{
    combat::{AttackClass, Projectile, ProjectileKind},
    game::{direction, Actor, World, DT, V3},
    motion::Motion,
};

pub const RIDE_SECONDS: f32 = 12.0;
pub const MOUNT_SECONDS: f32 = 2.0;
pub const ROAR_SECONDS: f32 = 2.0;
pub const DISMOUNT_SECONDS: f32 = 2.0;
pub const ATTACK_SECONDS: f32 = 0.8;
pub const ATTACK_RELEASE: f32 = 0.23;
pub const ROAR_RELEASE: f32 = 0.63;
pub const RIDE_LIFT: f32 = 1.65;
pub const ROAR_RADIUS: f32 = 6.0;
pub const ROAR_DAMAGE: f32 = 45.0;
pub const DOLL_COUNT: usize = 9;
pub const DOLL_SECONDS: f32 = 5.0;
pub const CONFETTI_SECONDS: f32 = 0.7;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Phase {
    #[default]
    Off,
    Mount,
    Ride,
    Roar,
    Dismount,
    Attack,
    RideAttack,
}
impl Phase {
    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => Self::Off,
            1 => Self::Mount,
            2 => Self::Ride,
            3 => Self::Roar,
            4 => Self::Dismount,
            5 => Self::Attack,
            6 => Self::RideAttack,
            _ => return None,
        })
    }
    pub fn duration(self) -> f32 {
        match self {
            Self::Off | Self::Ride => 0.0,
            Self::Attack | Self::RideAttack => ATTACK_SECONDS,
            Self::Mount => MOUNT_SECONDS,
            Self::Roar => ROAR_SECONDS,
            Self::Dismount => DISMOUNT_SECONDS,
        }
    }
    pub fn motion(self) -> Option<Motion> {
        Some(match self {
            Self::Off | Self::Ride => return None,
            Self::Mount => Motion::ChloeMount,
            Self::Roar => Motion::ChloeRoar,
            Self::Dismount => Motion::ChloeDismount,
            Self::Attack => Motion::ChloeAttack,
            Self::RideAttack => Motion::ChloeRideAttack,
        })
    }
    pub fn basic(self) -> bool {
        matches!(self, Self::Attack | Self::RideAttack)
    }
}
#[derive(Debug, Default, Clone, Copy)]
pub struct State {
    pub phase: Phase,
    pub elapsed: f32,
    pub remaining: f32,
    pub hits: u8,
}
impl State {
    pub fn active(self) -> bool {
        self.phase != Phase::Off
    }
    pub fn riding(self) -> bool {
        matches!(self.phase, Phase::Ride | Phase::RideAttack | Phase::Roar)
    }
    pub fn locked(self) -> bool {
        matches!(self.phase, Phase::Mount | Phase::Roar | Phase::Dismount)
    }
    pub fn busy(self) -> bool {
        self.locked() || self.phase.basic()
    }
    pub fn lift(self) -> f32 {
        match self.phase {
            Phase::Mount => RIDE_LIFT * smooth((self.elapsed - 0.25) / 0.80),
            Phase::Dismount => dismount_lift(self.elapsed),
            _ => {
                if self.riding() {
                    RIDE_LIFT
                } else {
                    0.0
                }
            }
        }
    }
    pub fn pack(self) -> u32 {
        let age = if self.phase == Phase::Ride {
            0
        } else {
            (self.elapsed / DT).round() as u32
        };
        age.min(127)
            | (((self.remaining / DT).round() as u32).min(720) << 7)
            | ((self.hits.min(8) as u32) << 17)
    }
    pub fn unpack(phase: Phase, bits: u32) -> Option<Self> {
        let elapsed = (bits & 127) as f32 * DT;
        let remaining = ((bits >> 7) & 1023) as f32 * DT;
        let hits = ((bits >> 17) & 15) as u8;
        if bits >> 21 != 0 || hits > 8 || remaining > RIDE_SECONDS + 0.001 {
            return None;
        }
        if matches!(phase, Phase::Off | Phase::Ride) && elapsed > 0.0
            || !matches!(phase, Phase::Off | Phase::Ride) && elapsed >= phase.duration()
        {
            return None;
        }
        if matches!(phase, Phase::Off | Phase::Attack | Phase::Dismount)
            && (remaining > 0.0 || hits > 0)
        {
            return None;
        }
        Some(Self {
            phase,
            elapsed,
            remaining,
            hits,
        })
    }
}
pub fn smooth(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    v * v * (3.0 - 2.0 * v)
}
pub fn dismount_lift(age: f32) -> f32 {
    let t = ((age - 0.30) / 0.95).clamp(0.0, 1.0);
    RIDE_LIFT * (1.0 - smooth(t)) + 0.36 * (t * std::f32::consts::PI).sin()
}
/// Only the rider's head earns a critical hit; Sebastian's head remains a body hit.
pub fn head_center(a: &Actor) -> V3 {
    a.pos
        + V3::new(
            0.0,
            (1.19 + a.combat.chloe.lift() - a.crouch_drop()) * a.body_scale(),
            0.0,
        )
        - direction(a.yaw, 0.0) * (0.64 * a.combat.chloe.lift() / RIDE_LIFT * a.body_scale())
}
fn head_ray(a: &Actor, origin: V3, dir: V3, padding: f32) -> Option<f32> {
    if a.character != 6 || a.combat.chloe.lift() < 0.05 {
        return None;
    }
    let delta = head_center(a) - origin;
    let t = delta.dot(dir);
    let radius = 0.27 * a.body_scale() + padding;
    let discriminant = radius * radius - (delta.dot(delta) - t * t);
    if discriminant < 0.0 || t + discriminant.sqrt() < 0.0 {
        return None;
    }
    Some((t - discriminant.sqrt()).max(0.0))
}
pub fn headshot(a: &Actor, origin: V3, dir: V3, distance: f32, padding: f32) -> bool {
    head_ray(a, origin, dir, padding).is_some_and(|t| t <= distance + 0.002)
}
/// The rider sits behind Sebastian. Include that offset for side-on shots as well.
pub fn ray_hit(a: &Actor, origin: V3, dir: V3, padding: f32) -> Option<f32> {
    use crate::game::ray_box;
    let scale = a.body_scale();
    let pad = V3::new(padding, padding, padding);
    let mounted = a.character == 6 && a.combat.chloe.lift() > 0.05;
    let height = if mounted {
        a.hit_height() - a.combat.chloe.lift() * scale
    } else {
        a.hit_height()
    };
    let body = ray_box(
        origin,
        dir,
        a.pos + V3::new(-0.48 * scale, 0.0, -0.48 * scale) - pad,
        a.pos + V3::new(0.48 * scale, height, 0.48 * scale) + pad,
    );
    if !mounted {
        return body;
    }
    let center = head_center(a);
    let rider = ray_box(
        origin,
        dir,
        center + V3::new(-0.30, -0.70, -0.30) * scale - pad,
        center + V3::new(0.30, -0.20, 0.30) * scale + pad,
    );
    [body, rider, head_ray(a, origin, dir, padding)]
        .into_iter()
        .flatten()
        .min_by(f32::total_cmp)
}
pub fn view_hands(m: Motion, age: f32) -> bool {
    matches!(
        m,
        Motion::ChloeRideIdle | Motion::ChloeRideMove | Motion::ChloeRideAttack | Motion::ChloeRoar
    ) || m == Motion::ChloeMount && age >= 0.35
        || m == Motion::ChloeDismount && age < 1.25
}
/// Blend the bear's view arms independently of Chloe's visible rider hands.
pub fn view_ride(m: Motion, age: f32) -> f32 {
    match m {
        Motion::ChloeMount => smooth((age - 0.25) / 0.80),
        Motion::ChloeDismount => 1.0 - smooth((age - 0.30) / 0.95),
        Motion::ChloeRideIdle
        | Motion::ChloeRideMove
        | Motion::ChloeRideAttack
        | Motion::ChloeRoar => 1.0,
        _ => 0.0,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ViewArm {
    pub tip: [f32; 3],
    pub corner: [f32; 3],
    pub scale: f32,
    pub roll: f32,
}

/// A wide rising swing from the lower screen corner, timed to the damage release.
pub fn view_arm(m: Motion, age: f32, left: bool) -> ViewArm {
    let side = if left { 1.0 } else { -1.0 };
    let ride = view_ride(m, age);
    let attack = matches!(m, Motion::ChloeAttack | Motion::ChloeRideAttack) && !left;
    let windup = if attack {
        smooth(age / 0.10) * (1.0 - smooth((age - 0.10) / 0.13))
    } else {
        0.0
    };
    let swing = if attack {
        smooth((age - 0.10) / (ATTACK_RELEASE - 0.10))
            * (1.0 - smooth((age - 0.33) / (ATTACK_SECONDS - 0.33)))
    } else {
        0.0
    };
    let bob = if matches!(m, Motion::Walk | Motion::Run | Motion::ChloeRideMove) {
        (age * 8.0 + if left { std::f32::consts::PI } else { 0.0 }).sin() * 0.025
    } else {
        0.0
    };
    ViewArm {
        tip: [
            side * (0.64 + 0.12 * ride + 0.10 * windup - (0.46 + 0.08 * ride) * swing),
            -0.34 - 0.02 * ride - 0.08 * windup + (0.38 + 0.08 * ride) * swing + bob,
            0.96 + 0.10 * ride - 0.10 * windup + 0.22 * swing,
        ],
        corner: [side * (1.02 + 0.22 * ride), -0.78 - 0.12 * ride, 0.10],
        scale: 1.0 + 0.38 * ride,
        roll: side * (0.04 + 0.20 * windup - 0.30 * swing),
    }
}
pub fn spread(age: f32) -> f32 {
    smooth((age - 1.25) / 0.16) * (1.0 - smooth((age - 1.70) / 0.30))
}
pub fn view_palm(m: Motion, age: f32, left: bool) -> [f32; 3] {
    let side = if left { 1.0 } else { -1.0 };
    // Rider hands rest inside the bear arms. Walking and attacking never drive them.
    let mut p = [side * 0.25, -0.38, 0.64];
    if m == Motion::ChloeMount {
        let w = spread(age);
        p = [side * (0.25 + 0.19 * w), -0.38 + 0.37 * w, 0.64];
    }
    if m == Motion::ChloeRoar {
        let w = smooth(age / 0.20) * (1.0 - smooth((age - 1.65) / 0.35));
        p = [side * (0.25 + 0.29 * w), -0.38 + 0.56 * w, 0.64 - 0.31 * w];
    }
    p
}
pub fn advance_actor(a: &mut Actor) {
    if a.character != 6 || !a.alive() || a.burrowed() || a.combat.polymorph > 0.0 {
        a.combat.chloe = State::default();
        return;
    }
    let dt = DT * crate::renua_skill::time_factor(a);
    let interrupted = a.combat.action_interrupted();
    let rate = crate::attack::rate(a);
    let s = &mut a.combat.chloe;
    // Hard control cancels a swing/roar but never grants a fresh ride timer.
    if interrupted && (s.phase.basic() || s.phase == Phase::Roar) {
        s.phase = if s.riding() { Phase::Ride } else { Phase::Off };
        s.elapsed = 0.0;
    }
    if s.riding() {
        s.remaining = (s.remaining - dt).max(0.0);
        if s.remaining <= 0.00001 {
            s.phase = Phase::Dismount;
            s.elapsed = 0.0;
            s.hits = 0;
            return;
        }
    }
    if matches!(s.phase, Phase::Off | Phase::Ride) {
        return;
    }
    s.elapsed += dt * if s.phase.basic() { rate } else { 1.0 };
    if s.elapsed + 0.00001 >= s.phase.duration() {
        let phase = s.phase;
        if matches!(phase, Phase::Mount | Phase::Roar | Phase::RideAttack) {
            s.phase = Phase::Ride;
            s.elapsed = 0.0;
            if phase == Phase::Mount {
                s.remaining = RIDE_SECONDS;
            }
        } else {
            *s = State::default();
        }
    }
}
impl World {
    pub(crate) fn start_chloe(&mut self, source: usize, mount: bool) {
        let a = &mut self.actors[source];
        if mount {
            a.combat.chloe = State {
                phase: Phase::Mount,
                ..Default::default()
            };
        } else {
            a.combat.chloe.phase = if a.combat.chloe.riding() {
                Phase::RideAttack
            } else {
                Phase::Attack
            };
            a.combat.chloe.elapsed = 0.0;
            a.shot = a.shot.wrapping_add(1);
            a.combat.attacks = a.combat.attacks.wrapping_add(1);
            a.cooldown = ATTACK_SECONDS / crate::attack::rate(a);
        }
        a.shield = 0.0;
        a.reload = 0.0;
        a.ability_flash = 0.0;
        a.dancing = false;
        a.wave_ticks = 0;
        if mount {
            a.race_skill.cancel();
            a.portals.momentum = V3::default();
        }
    }
    pub(crate) fn advance_chloe(&mut self, source: usize) {
        let before = self.actors[source].combat.chloe;
        advance_actor(&mut self.actors[source]);
        let after = self.actors[source].combat.chloe;
        if before.phase != after.phase || self.actors[source].combat.action_interrupted() {
            return;
        }
        let crossed = |t| before.elapsed + 0.00001 < t && after.elapsed + 0.00001 >= t;
        if after.phase.basic() && crossed(ATTACK_RELEASE) {
            let a = &self.actors[source];
            let origin = a.eye();
            let dir = direction(a.yaw, a.pitch);
            let riding = after.riding();
            let range = if riding { 4.4 } else { 3.8 };
            let mut connected = false;
            // Aim by the torso when the rider is above a nearby standing opponent.
            for target in self.targets(source, origin, range, false) {
                let diff = self.actors[target].eye() - origin;
                if diff.flat().normalized().dot(dir.flat().normalized()) < 0.5 {
                    continue;
                }
                if self.damage_hit(
                    source,
                    target,
                    if riding { 40.0 } else { 27.0 },
                    AttackClass::Basic,
                    6,
                ) {
                    connected = true;
                    self.push(target, dir, 1.3);
                }
            }
            self.actors[source].shot_end = origin + dir * range;
            if let Some(hit) = crate::impact::melee_contact(
                &self.map,
                origin,
                self.actors[source].yaw,
                self.actors[source].pitch,
                range,
            ) {
                let a = &self.actors[source];
                let tilt = if a.combat.attacks.is_multiple_of(2) {
                    -0.55
                } else {
                    0.55
                };
                let rotation = crate::impact::slash_rotation(hit.face, a.yaw, a.pitch, tilt);
                self.record_slash(hit, 0.48 * self.actors[source].body_scale(), rotation);
            }
            if riding && connected {
                let s = &mut self.actors[source].combat.chloe;
                s.hits += 1;
                if s.hits >= 9 {
                    s.hits = 0;
                    s.remaining = RIDE_SECONDS;
                    s.phase = Phase::Roar;
                    s.elapsed = 0.0;
                    self.actors[source].race_skill.cancel();
                    self.actors[source].portals.momentum = V3::default();
                }
            }
        } else if after.phase == Phase::Roar && crossed(ROAR_RELEASE) {
            let center = self.actors[source].pos + V3::new(0.0, 1.2, 0.0);
            for target in self.targets(source, center, ROAR_RADIUS, false) {
                self.damage_hit(source, target, ROAR_DAMAGE, AttackClass::Skill, 6);
            }
        }
    }
    pub(crate) fn summon_sebastians(&mut self, source: usize) {
        self.actors[source].ability_flash = 0.0;
        self.actors[source].combat.ultimate_flash = 0.0;
        let a = &self.actors[source];
        let origin = a.pos;
        let yaw = a.yaw;
        let forward = direction(yaw, 0.0);
        let right = direction(yaw + std::f32::consts::FRAC_PI_2, 0.0);
        for n in 0..DOLL_COUNT {
            if let Some(i) = self.projectile(
                source,
                ProjectileKind::Doll,
                direction(yaw + (n as f32 - 4.0) * 0.17, 0.0),
                7.0,
                14.0,
                AttackClass::Ultimate,
            ) {
                let p = &mut self.projectiles[i];
                p.pos = origin - forward * (0.65 + (n / 3) as f32 * 0.28)
                    + right * ((n % 3) as f32 - 1.0) * 0.36
                    + V3::new(0.0, 0.45, 0.0);
                p.origin = p.pos;
                p.life = DOLL_SECONDS;
            }
        }
    }
    pub(crate) fn chloe_confetti(&mut self, p: &Projectile) {
        let mut fx = p.clone();
        fx.kind = ProjectileKind::Confetti;
        fx.age = 0.0;
        fx.life = CONFETTI_SECONDS;
        fx.velocity = V3::default();
        fx.damage = 0.0;
        self.projectiles.push(fx);
    }
}
