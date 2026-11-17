//! Bounded local combat recording. Playback changes presentation, never the live simulation.
use crate::{
    combat::Projectile,
    game::{direction, ray_box, team, Actor, Phase, World, DT, PLAYERS, V3},
    level::MapId,
    motion::{death_seconds, interpolate_angle, MotionClock, Pose},
};
use std::collections::VecDeque;
#[cfg(test)]
mod tests;
pub const REPLAY_SECONDS: f32 = 3.0;
const HISTORY_TICKS: u64 = 180;
const MAX_FRAMES: usize = 182;
pub const DEATH_HOLD: f32 = 0.15;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    #[default]
    Inactive,
    TopDown,
    Replay,
    Finished,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeathView {
    pub target: usize,
    pub position: V3,
    pub yaw: f32,
    pub pitch: f32,
    pub frozen: bool,
}
#[derive(Debug, Clone)]
struct Frame {
    tick: u64,
    actors: [Actor; PLAYERS],
    projectiles: Vec<Projectile>,
    impacts: Vec<crate::impact::Impact>,
    poses: [Pose; PLAYERS],
    point: usize,
    progress: f32,
    remaining: f32,
    contested: bool,
    scores: [u16; 2],
}
#[derive(Debug, Default)]
pub struct DeathCamera {
    pub view: Option<DeathView>,
    pub stage: Stage,
    pub elapsed: f32,
    pub replay_time: f32,
    pub replay_duration: f32,
    key: Option<(MapId, u16, usize)>,
    death: Option<(usize, u16, u16)>,
    history: VecDeque<Frame>,
    clip: Vec<Frame>,
    clocks: [MotionClock; PLAYERS],
    scene: Option<World>,
    poses: [Pose; PLAYERS],
    victim: Option<Actor>,
    killer: Option<usize>,
}
impl DeathCamera {
    pub fn scene(&self) -> Option<&World> {
        self.scene.as_ref()
    }
    pub fn poses(&self) -> &[Pose; PLAYERS] {
        &self.poses
    }
    pub fn is_replay(&self) -> bool {
        matches!(self.stage, Stage::Replay | Stage::Finished)
    }
    pub fn death_duration(&self) -> f32 {
        self.victim
            .as_ref()
            .map_or(0.0, |v| death_seconds(v.character))
    }
    pub fn update(&mut self, world: &World, slot: usize, dt: f32) -> Option<DeathView> {
        let Some(victim) = world.actors.get(slot) else {
            *self = Self::default();
            return None;
        };
        let key = (world.map_id, world.round, slot);
        if world.phase != Phase::Playing
            || self.key != Some(key)
            || self.history.back().is_some_and(|f| world.tick < f.tick)
        {
            *self = Self::default();
            self.key = Some(key);
        }
        if world.phase != Phase::Playing {
            return None;
        }
        if self.history.back().is_none_or(|f| f.tick != world.tick) {
            let step = self
                .history
                .back()
                .map_or(0.0, |f| (world.tick - f.tick) as f32 * DT);
            let poses = std::array::from_fn(|i| {
                self.clocks[i].observe(
                    &world.actors[i],
                    step,
                    world.tick,
                    crate::movement::grounded(&world.map, world.actors[i].pos),
                )
            });
            self.history.push_back(Frame {
                tick: world.tick,
                actors: world.actors.clone(),
                projectiles: world.projectiles.clone(),
                impacts: world.impacts.clone(),
                poses,
                point: world.point,
                progress: world.progress,
                remaining: world.remaining,
                contested: world.contested,
                scores: world.scores,
            });
            while self.history.len() > MAX_FRAMES
                || self
                    .history
                    .front()
                    .is_some_and(|f| world.tick - f.tick > HISTORY_TICKS)
            {
                self.history.pop_front();
            }
        }
        if victim.alive() || !victim.active {
            self.view = None;
            self.stage = Stage::Inactive;
            self.death = None;
            self.scene = None;
            self.victim = None;
            self.clip.clear();
            return None;
        }
        let death = (slot, world.round, victim.deaths);
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        if self.death != Some(death) {
            self.death = Some(death);
            self.stage = Stage::TopDown;
            self.elapsed = 0.0;
            self.replay_time = 0.0;
            self.victim = Some(victim.clone());
            self.killer = victim
                .killed_by
                .filter(|&i| i < PLAYERS && team(i) != team(slot));
            self.clip = self.history.iter().cloned().collect();
            // Include the lethal packet even if it reused the previous tick.
            if let Some(last) = self.clip.last_mut() {
                last.actors = world.actors.clone();
                last.projectiles = world.projectiles.clone();
                last.impacts = world.impacts.clone();
                last.poses[slot] = Pose {
                    motion: crate::motion::Motion::Death,
                    age: 0.0,
                    locomotion: None,
                    ..last.poses[slot]
                };
            }
            self.replay_duration = self
                .clip
                .first()
                .map_or(0.0, |f| (world.tick - f.tick) as f32 * DT);
            self.scene = Some(world.clone());
        } else {
            self.elapsed += dt;
        }
        let top_duration = self.death_duration() + DEATH_HOLD;
        if self.elapsed < top_duration || self.killer.is_none() || self.replay_duration <= DT {
            self.stage = Stage::TopDown;
            let dead = self.victim.as_ref().unwrap();
            let scene = self.scene.as_mut().unwrap();
            scene.clone_from(world);
            // Keep the death-time hero/body if the next hero is selected while dead.
            scene.actors[slot] = dead.clone();
            self.poses = self.history.back().unwrap().poses;
            self.poses[slot] = Pose {
                motion: crate::motion::Motion::Death,
                age: self.elapsed,
                shot_age: 100.0,
                locomotion: None,
            };
            let yaw = dead.yaw + std::f32::consts::PI;
            let pitch = -1.21;
            let focus = dead.pos + V3::new(0.0, 0.5 * dead.body_scale(), 0.0);
            let back = direction(yaw, pitch) * -1.0;
            let padding = V3::new(0.12, 0.12, 0.12);
            let mut distance = 5.6 * dead.body_scale();
            for block in &world.map {
                if let Some(hit) =
                    ray_box(focus, back, block.min() - padding, block.max() + padding)
                {
                    // A ghost can die inside a wall; do not collapse the camera onto its body.
                    if hit > 0.2 {
                        distance = distance.min((hit - 0.04).max(0.0));
                    }
                }
            }
            self.view = Some(DeathView {
                target: slot,
                position: focus + back * distance,
                yaw,
                pitch,
                frozen: false,
            });
        } else {
            self.replay_time = (self.elapsed - top_duration).min(self.replay_duration);
            self.stage = if self.replay_time >= self.replay_duration {
                Stage::Finished
            } else {
                Stage::Replay
            };
            self.sample();
            let target = self.killer.unwrap();
            let actor = &self.scene.as_ref().unwrap().actors[target];
            self.view = Some(DeathView {
                target,
                position: actor.eye(),
                yaw: actor.yaw,
                pitch: actor.pitch,
                frozen: self.stage == Stage::Finished,
            });
        }
        self.view
    }
    fn sample(&mut self) {
        let tick = if self.stage == Stage::Finished {
            self.clip.last().unwrap().tick as f64
        } else {
            self.clip[0].tick as f64 + (self.replay_time / DT) as f64
        };
        let next = self.clip.partition_point(|f| f.tick as f64 <= tick);
        let a = &self.clip[next.saturating_sub(1).min(self.clip.len() - 1)];
        let b = &self.clip[next.min(self.clip.len() - 1)];
        let t = if b.tick == a.tick {
            0.0
        } else {
            ((tick - a.tick as f64) / (b.tick - a.tick) as f64).clamp(0.0, 1.0) as f32
        };
        let scene = self.scene.as_mut().unwrap();
        scene.tick = tick.floor() as u64;
        scene.point = a.point;
        scene.progress = a.progress;
        scene.remaining = a.remaining;
        scene.contested = a.contested;
        scene.scores = a.scores;
        scene.actors.clone_from(&a.actors);
        self.poses = a.poses;
        for (i, actor) in scene.actors.iter_mut().enumerate() {
            let later = &b.actors[i];
            if actor.character == later.character
                && actor.deaths == later.deaths
                && actor.alive() == later.alive()
                && actor.human == later.human
                && (actor.pos - later.pos).length() < 4.0
            {
                actor.pos = actor.pos + (later.pos - actor.pos) * t;
                actor.yaw = interpolate_angle(actor.yaw, later.yaw, t);
                actor.pitch += (later.pitch - actor.pitch) * t;
            }
            self.poses[i].age += (tick - a.tick as f64) as f32
                * DT
                * if self.poses[i].motion.is_attack() {
                    crate::attack::rate(actor)
                } else {
                    1.0
                };
            self.poses[i].shot_age += (tick - a.tick as f64) as f32 * DT;
            if let Some(movement) = &mut self.poses[i].locomotion {
                movement.age = (movement.age + (tick - a.tick as f64) as f32 * DT)
                    .rem_euclid(movement.motion.seconds(actor.character));
            }
        }
        scene.projectiles.clone_from(&a.projectiles);
        scene.impacts.clone_from(&a.impacts);
        for p in &mut scene.projectiles {
            if let Some(later) = b.projectiles.iter().find(|q| q.id == p.id) {
                p.pos = p.pos + (later.pos - p.pos) * t;
            }
        }
    }
}
