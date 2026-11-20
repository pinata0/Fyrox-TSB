//! Small, versioned UDP protocol for a LAN prototype.

use crate::chat::{
    clean_text, ChatChannel, ChatHistory, ChatMessage, CHAT_HISTORY, MAX_CHAT_BYTES,
};
use crate::communication::{Communication, CommunicationResult, WAVE_TICKS};
use crate::game::*;
use crate::impact::{Impact, ImpactKind, IMPACT_SECONDS, MAX_IMPACTS};
use crate::online::{unix_now, HostGate, PlayerIdentity};
use crate::prediction::ClientMotion;
use crate::roster::{SelectionResult, ROSTER};
use crate::transport::{PeerAddr, RoomCode, Socket, MAX_DATAGRAM};
use crate::voice::{Samples, VoiceFrame, VoiceInbox, VoiceRate, FRAME_SAMPLES};
use crate::{
    combat::{Projectile, ProjectileKind, MAX_PROJECTILES},
    kits::kit,
};
use std::{
    collections::{HashSet, VecDeque},
    io,
    net::{SocketAddr, ToSocketAddrs},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MAGIC: &[u8; 4] = b"TSBO";
const HELLO: u8 = 1;
#[cfg(test)]
mod ui_tests;
#[cfg(test)]
mod vivi_tests;
const WELCOME: u8 = 2;
const INPUT: u8 = 3;
const SNAPSHOT: u8 = 4;
const FULL: u8 = 5;
const SELECT: u8 = 6;
const SELECTED: u8 = 7;
const PROJECTILES: u8 = 8;
const CHAT_SEND: u8 = 9;
const CHAT_ACCEPTED: u8 = 10;
const CHAT_DELIVERY: u8 = 11;
const CHAT_RECEIVED: u8 = 12;
const COMMUNICATE: u8 = 13;
const COMMUNICATED: u8 = 14;
const VOICE_SEND: u8 = 15;
const VOICE_DELIVERY: u8 = 16;
const INVITED_HELLO: u8 = 17;
const DENIED: u8 = 18;
const PLAYER_NAME: u8 = 19;
const IMPACTS: u8 = 20;
const PORTALS: u8 = 21;
const MATCH_STATS: u8 = 22;
#[cfg(test)]
#[path = "net/aya_tests.rs"]
mod aya_tests;
mod scoreboard;
use scoreboard::{read_stats, stats_snapshot};
const CHAT_RETRY: Duration = Duration::from_millis(200);
const TIMEOUT: Duration = Duration::from_secs(8);
const MAX_PACKET: usize = MAX_DATAGRAM;
#[cfg(test)]
#[path = "net/dragon_charge_tests.rs"]
mod dragon_charge_tests;
#[cfg(test)]
#[path = "net/id_tests.rs"]
mod id_tests;
#[cfg(test)]
#[path = "net/portal_tests.rs"]
mod portal_tests;
#[cfg(test)]
#[path = "net/race_skill_tests.rs"]
mod race_skill_tests;
#[cfg(test)]
use std::net::UdpSocket;

#[derive(Debug)]
struct Writer(Vec<u8>);
impl Writer {
    fn new(kind: u8) -> Self {
        let mut w = Self(MAGIC.to_vec());
        w.u8(kind);
        w
    }
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend(v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend(v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend(v.to_le_bytes());
    }
    fn v3(&mut self, v: V3) {
        self.f32(v.x);
        self.f32(v.y);
        self.f32(v.z);
    }
    fn text(&mut self, text: &str) {
        self.u16(text.len() as u16);
        self.0.extend_from_slice(text.as_bytes());
    }
}
struct Reader<'a>(&'a [u8], usize);
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], kind: u8) -> Option<Self> {
        (bytes.len() <= MAX_PACKET && bytes.get(..4)? == MAGIC && *bytes.get(4)? == kind)
            .then_some(Self(bytes, 5))
    }
    fn bytes<const N: usize>(&mut self) -> Option<[u8; N]> {
        let r = self.0.get(self.1..self.1 + N)?.try_into().ok()?;
        self.1 += N;
        Some(r)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.bytes::<1>()?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.bytes()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.bytes()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.bytes()?))
    }
    fn f32(&mut self) -> Option<f32> {
        let v = f32::from_le_bytes(self.bytes()?);
        v.is_finite().then_some(v)
    }
    fn v3(&mut self) -> Option<V3> {
        Some(V3::new(self.f32()?, self.f32()?, self.f32()?))
    }
    fn done(&self) -> bool {
        self.1 == self.0.len()
    }
    fn text(&mut self) -> Option<String> {
        let len = self.u16()? as usize;
        if len > MAX_CHAT_BYTES {
            return None;
        }
        let text = std::str::from_utf8(self.0.get(self.1..self.1 + len)?).ok()?;
        self.1 += len;
        let cleaned = clean_text(text);
        (!cleaned.is_empty() && cleaned == text).then_some(cleaned)
    }
    fn invitation(&mut self) -> Option<String> {
        let len = self.u16()? as usize;
        if len >= 512 {
            return None;
        }
        let text = std::str::from_utf8(self.0.get(self.1..self.1 + len)?).ok()?;
        self.1 += len;
        text.is_ascii().then(|| text.to_owned())
    }
}

fn chat_request(token: u64, sequence: u64, channel: ChatChannel, text: &str) -> Vec<u8> {
    let mut w = Writer::new(CHAT_SEND);
    w.u64(token);
    w.u64(sequence);
    w.u8(channel as u8);
    w.text(text);
    w.0
}

fn read_chat_request(bytes: &[u8]) -> Option<(u64, u64, ChatChannel, String)> {
    let mut r = Reader::new(bytes, CHAT_SEND)?;
    let result = (
        r.u64()?,
        r.u64()?,
        ChatChannel::from_wire(r.u8()?)?,
        r.text()?,
    );
    r.done().then_some(result)
}

fn chat_delivery(token: u64, message: &ChatMessage) -> Vec<u8> {
    let mut w = Writer::new(CHAT_DELIVERY);
    w.u64(token);
    w.u64(message.id);
    w.u8(message.sender as u8);
    w.u8(message.character);
    w.u8(message.channel as u8);
    w.text(&message.text);
    if message.voice_line {
        w.u8(1); // Optional server-owned cue; ordinary chat keeps its wire encoding.
    }
    w.0
}

fn read_chat_delivery(bytes: &[u8], token: u64) -> Option<ChatMessage> {
    let mut r = Reader::new(bytes, CHAT_DELIVERY)?;
    if r.u64()? != token {
        return None;
    }
    let mut message = ChatMessage {
        id: r.u64()?,
        sender: r.u8()? as usize,
        character: r.u8()?,
        channel: ChatChannel::from_wire(r.u8()?)?,
        text: r.text()?,
        voice_line: false,
    };
    if !r.done() {
        if r.u8()? != 1 || message.channel != ChatChannel::Team {
            return None;
        }
        message.voice_line = true;
    }
    (r.done()
        && message.id > 0
        && message.sender < PLAYERS
        && (message.character as usize) < ROSTER.len())
    .then_some(message)
}

fn chat_ack(kind: u8, token: u64, sequence: u64) -> Vec<u8> {
    let mut w = Writer::new(kind);
    w.u64(token);
    w.u64(sequence);
    w.0
}

fn read_chat_ack(bytes: &[u8], kind: u8) -> Option<(u64, u64)> {
    let mut r = Reader::new(bytes, kind)?;
    let result = (r.u64()?, r.u64()?);
    r.done().then_some(result)
}

fn communication_request(token: u64, sequence: u64, action: Communication) -> Vec<u8> {
    let mut w = Writer::new(COMMUNICATE);
    w.u64(token);
    w.u64(sequence);
    w.u8(action as u8);
    w.0
}
fn read_communication(bytes: &[u8]) -> Option<(u64, u64, Communication)> {
    let mut r = Reader::new(bytes, COMMUNICATE)?;
    let result = (r.u64()?, r.u64()?, Communication::from_wire(r.u8()?)?);
    r.done().then_some(result)
}

#[derive(Debug, Clone, Copy)]
struct InputAck {
    slot: usize,
    sequence: u64,
}

#[cfg(test)]
fn snapshot(world: &World, token: u64, seq: u64) -> Vec<u8> {
    peer_snapshot(
        world,
        token,
        seq,
        InputAck {
            slot: 0,
            sequence: 0,
        },
    )
}

fn peer_snapshot(world: &World, token: u64, seq: u64, ack: InputAck) -> Vec<u8> {
    let mut w = Writer::new(SNAPSHOT);
    w.u64(token);
    w.u64(seq);
    w.u64(world.tick);
    w.u8(world.map_id as u8);
    w.u8(world.phase as u8);
    w.f32(world.phase_time);
    w.u16(world.round);
    w.u8(world.attacker as u8);
    w.u8(world.point as u8);
    w.f32(world.progress);
    w.f32(world.peak);
    w.f32(world.remaining);
    w.u8(world.overtime as u8);
    w.f32(world.overtime_left);
    w.u8(world.contested as u8);
    for n in world.scores {
        w.u16(n);
    }
    for n in world.banks {
        w.f32(n);
    }
    w.u8(world.result.map_or(255, |n| n as u8));
    w.u8(world.extra as u8);
    w.u8(world.free_attack as u8);
    for a in &world.actors {
        w.u8(a.character);
        w.u8(a.combat.ultimate.clamp(0.0, 100.0).floor() as u8);
        w.u16(a.combat.flags());
        w.u8(a.combat.guard.round() as u8);
        // Preserve the MTU: three ammo bits, two combo bits, three Naia phase bits.
        let combo = if a.character == 4 {
            a.combat.aya_combo
        } else {
            a.combat.vivi_combo
        };
        let action = if a.character == 6 {
            a.combat.chloe.phase as u8
        } else if a.character == 0 {
            a.combat.vivi.phase as u8
        } else if a.character == 2 {
            if a.combat.erpin_skill.active() { 3 } else { (a.combat.erpin.eating as u8)
                | ((a.combat.erpin.shot_delay > 0.0) as u8) << 1
                | (a.combat.erpin.alt_down as u8) << 2 }
        } else if a.character == 5 {
            a.combat.picora.phase as u8
        } else if a.character == 4 {
            a.combat.aya.phase as u8
        } else if a.character == 7 {
            a.combat.renua.phase as u8
        } else if a.character == 9 {
            a.combat.bella.phase as u8
        } else if a.character == 8 {
            a.combat.mayo.phase as u8
        } else if a.character == 10 {
            a.combat.ui.phase as u8
        } else if a.character == 11 {
            a.combat.butter.phase as u8
        } else if a.character == 12 {
            a.combat.komi.phase as u8
        } else if a.character == 13 {
            a.combat.hilde.phase as u8
        } else if a.character == 14 {
            a.combat.shion.phase as u8
        } else if a.character == 3 {
            a.combat.id.phase as u8
        } else {
            a.combat.naia.phase as u8
        };
        // Ui does not use magic ammo. Reuse those three bits for her basic
        // cycle (0=never fired, 1..4), including the fourth-shot ray visual.
        let attack_bits = if a.character == 10 {
            if a.combat.attacks == 0 { 0 } else { ((a.combat.attacks - 1) % 4 + 1) as u8 }
        } else { a.combat.magic_ammo };
        w.u8(attack_bits | (combo << 3) | (action << 5));
        w.f32(a.barrier);
        w.u16(
            a.race_skill.surge_charges as u16
                | (((a.race_skill.surge_recharge / DT).round() as u16) << 2),
        );
        // Race-discriminated Shift state occupies the same four bytes for all heroes.
        if matches!(
            crate::roster::character(a.character).race,
            crate::roster::Race::Beast
                | crate::roster::Race::Spirit
                | crate::roster::Race::Dragon
                | crate::roster::Race::Fairy
                | crate::roster::Race::Witch
        ) {
            let s = a.race_skill;
            w.u8((s.charge / DT).round() as u8 | (s.fire_latched as u8) << 7);
            w.u8((s.remaining / DT).round() as u8);
            w.u16((s.cooldown / DT).round() as u16 | (s.down as u16) << 15);
        } else {
            w.u8((a.ghost_phase / DT).round() as u8);
            w.u8((a.phase_skill.remaining / DT).round() as u8);
            w.u16((a.phase_skill.cooldown / DT).round() as u16 | (a.phase_skill.down as u16) << 15);
        }
        w.f32(a.ability_cooldown);
        // During Naia E the phase owns the visual clock; its age shares the flash slot.
        if a.character == 6 {
            w.u32(a.combat.chloe.pack());
        } else if a.character == 0 {
            w.u32(a.combat.vivi.pack());
        } else if a.character == 2 && a.combat.erpin_skill.active() {
            w.u32(a.combat.erpin_skill.pack(a.combat.erpin.alt_down));
        } else if a.character == 5 {
            w.u32(a.combat.picora.pack());
        } else if a.character == 4 {
            w.u32(a.combat.aya.pack());
        } else if a.character == 7 {
            w.u32(a.combat.renua.pack());
        } else if a.character == 9 {
            w.u32(a.combat.bella.pack());
        } else if a.character == 8 {
            w.u32(a.combat.mayo.pack());
        } else if a.character == 10 {
            w.u32(a.combat.ui.pack());
        } else if a.character == 11 {
            w.u32(a.combat.butter.pack());
        } else if a.character == 12 {
            w.u32(a.combat.komi.pack());
        } else if a.character == 13 {
            w.u32(a.combat.hilde.pack());
        } else if a.character == 14 {
            w.u32(a.combat.shion.pack());
        } else if a.character == 3 {
            w.u32(a.combat.id.pack());
        } else {
            w.f32(if a.combat.erpin.eating {
                a.combat.erpin.elapsed
            } else if a.combat.erpin.shot_delay > 0.0 {
                a.combat.erpin.shot_delay
            } else if a.combat.naia.active() {
                a.combat.naia.elapsed
            } else {
                a.ability_flash
            });
        }
        w.v3(a.pos);
        w.f32(a.yaw);
        w.f32(a.pitch);
        // 8 HP bits (maximum 240), two 4-bit buff timers; same snapshot size.
        let sticker = (a.combat.sticker / crate::picora_skill::STICKER_SECONDS * 15.0).ceil().clamp(0.0, 15.0) as u16;
        let fashion = (a.combat.fashion / crate::picora_skill::FASHION_SECONDS * 15.0).ceil().clamp(0.0, 15.0) as u16;
        w.u16(a.hp.min(255) | (sticker << 8) | (fashion << 12));
        w.u8(a.ammo);
        w.f32(a.reload);
        w.f32(a.respawn);
        w.f32(a.shield);
        w.u16(a.kills);
        w.u16(a.deaths);
        // The high nibble stores killer slot + 1 (zero means no killer), keeping the MTU.
        w.u8(a.human as u8
            | (a.dancing as u8) << 1
            | ((a.combat.regen > 0.0) as u8) << 2
            | (a.active as u8) << 3
            | (a.killed_by.map_or(0, |slot| slot as u8 + 1) << 4));
        // Greeting lasts at most 108 ticks; the high bit carries crouch without growing the packet.
        w.u8(a.wave_ticks | (a.crouching as u8) << 7);
        w.u32(a.shot);
        w.v3(a.shot_end);
        w.f32(a.hit);
        // Preserve the four-byte hurt slot and the full-roster datagram budget.
        // Seven hurt bits at 120 Hz, eight frog-duration ticks, one rain bit.
        w.u16((a.hurt.clamp(0.0,1.0)*120.0).round() as u16
            | (((a.combat.polymorph/DT).round().clamp(0.0,180.0) as u16)<<7)
            | ((a.combat.rain_slow>0.0) as u16)<<15);
        w.u8((a.combat.temporal.clamp(0.0, 1.0) * 255.0).round() as u8);
        w.u8((a.combat.frozen / DT).round().clamp(0.0, 180.0) as u8);
    }
    let event = world.event.as_bytes();
    // Reserve 35 bytes for this peer's movement acknowledgement and safe phase
    // exit position, retaining the
    // 1200-byte datagram limit (including a full twelve-player roster).
    let event_budget = (MAX_PACKET - 1).saturating_sub(w.0.len() + 1 + 35);
    let mut len = event.len().min(20).min(event_budget);
    while !world.event.is_char_boundary(len) {
        len -= 1;
    }
    w.u8(len as u8);
    w.0.extend(&event[..len]);
    w.u8(ack.slot as u8);
    w.u64(ack.sequence);
    let local = &world.actors[ack.slot];
    w.f32(local.vy);
    for timer in [
        local.combat.slow,
        local.combat.speed_up,
        local.combat.stun,
        local.combat.sleep,
        local.combat.dash,
    ] {
        w.u16((timer / DT).round().clamp(0.0, u16::MAX as f32) as u16);
    }
    w.v3(
        if crate::roster::character(local.character).race == crate::roster::Race::Beast
            || crate::race_skill::has_surge(crate::roster::character(local.character).race)
        {
            local.race_skill.velocity
        } else {
            local.phase_skill.safe_position
        },
    );
    debug_assert!(w.0.len() < MAX_PACKET);
    w.0
}
fn read_snapshot(bytes: &[u8], token: u64) -> Option<(u64, World, InputAck)> {
    let mut r = Reader::new(bytes, SNAPSHOT)?;
    if r.u64()? != token {
        return None;
    }
    let seq = r.u64()?;
    let tick = r.u64()?;
    let map_id = crate::level::MapId::from_wire(r.u8()?)?;
    let phase = match r.u8()? {
        0 => Phase::Lobby,
        1 => Phase::Setup,
        2 => Phase::Playing,
        3 => Phase::Intermission,
        4 => Phase::Finished,
        _ => return None,
    };
    let mut w = World {
        tick,
        phase,
        phase_time: r.f32()?,
        round: r.u16()?,
        attacker: r.u8()? as usize,
        point: r.u8()? as usize,
        progress: r.f32()?,
        peak: r.f32()?,
        remaining: r.f32()?,
        overtime: r.u8()? != 0,
        overtime_left: r.f32()?,
        contested: r.u8()? != 0,
        ..World::for_map(map_id)
    };
    if w.attacker > 1 || w.point > 1 || !(0.0..=1.0).contains(&w.progress) {
        return None;
    }
    for n in &mut w.scores {
        *n = r.u16()?;
    }
    for n in &mut w.banks {
        *n = r.f32()?;
    }
    w.result = match r.u8()? {
        255 => None,
        n @ 0..=2 => Some(n as usize),
        _ => return None,
    };
    w.extra = r.u8()? != 0;
    w.free_attack = r.u8()? != 0;
    for a in &mut w.actors {
        a.character = r.u8()?;
        if a.character as usize >= ROSTER.len() {
            return None;
        }
        a.combat.ultimate = r.u8()? as f32;
        a.combat.read_flags(r.u16()?);
        a.combat.guard = r.u8()? as f32;
        let attack_state = r.u8()?;
        a.combat.magic_ammo = attack_state & 7;
        if a.character == 10 {
            if a.combat.magic_ammo > 4 { return None; }
            a.combat.attacks = a.combat.magic_ammo as u32;
            a.combat.magic_ammo = 0;
        }
        let combo = (attack_state >> 3) & 3;
        a.combat.vivi_combo = if a.character == 0 { combo } else { 0 };
        a.combat.aya_combo = if a.character == 4 { combo } else { 0 };
        let action = attack_state >> 5;
        if a.character == 6 {
            a.combat.chloe.phase = crate::chloe_skill::Phase::from_wire(action)?;
        } else if a.character == 0 {
            a.combat.vivi.phase = crate::vivi_skill::Phase::from_wire(action)?;
            if combo > 2 { return None; }
        } else if a.character == 2 {
            a.combat.erpin.eating = action & 1 != 0;
            a.combat.erpin.alt_down = action & 4 != 0;
            if action == 7 { return None; }
            if action == 3 { a.combat.erpin.eating = false; }
        } else if a.character == 5 {
            a.combat.picora.phase = crate::picora_skill::Phase::from_wire(action)?;
        } else if a.character == 4 {
            a.combat.aya.phase = crate::aya_skill::Phase::from_wire(action)?;
        } else if a.character == 7 {
            a.combat.renua.phase = crate::renua_skill::Phase::from_wire(action)?;
        } else if a.character == 9 {
            a.combat.bella.phase = crate::bella_skill::Phase::from_wire(action)?;
        } else if a.character == 8 {
            a.combat.mayo.phase = crate::mayo_skill::Phase::from_wire(action)?;
        } else if a.character == 10 {
            a.combat.ui.phase = crate::ui_skill::Phase::from_wire(action)?;
        } else if a.character == 11 {
            a.combat.butter.phase = crate::butter_skill::Phase::from_wire(action)?;
        } else if a.character == 12 {
            a.combat.komi.phase = crate::komi_skill::Phase::from_wire(action)?;
        } else if a.character == 13 {
            a.combat.hilde.phase = crate::hilde_skill::Phase::from_wire(action)?;
        } else if a.character == 14 {
            a.combat.shion.phase = crate::shion_skill::Phase::from_wire(action)?;
        } else if a.character == 3 {
            a.combat.id.phase = crate::id_skill::Phase::from_wire(action)?;
        } else {
            a.combat.naia.phase = crate::naia_skill::Phase::from_wire(action)?;
        }
        if (a.character != 1 && a.combat.naia.active())
            || (!matches!(a.character, 0 | 4) && combo != 0)
        {
            return None;
        }
        if a.combat.ultimate > 100.0 || a.combat.guard > 120.0 || a.combat.magic_ammo > 5 {
            return None;
        }
        a.barrier = r.f32()?;
        let surge = r.u16()?;
        let surge_charges = (surge & 3) as u8;
        let recharge_ticks = surge >> 2;
        if recharge_ticks > 180
            || (surge_charges == crate::race_skill::SURGE_CHARGES) != (recharge_ticks == 0)
        {
            return None;
        }
        let contact_ticks = r.u8()?;
        let phase_ticks = r.u8()?;
        let phase_flags = r.u16()?;
        let cooldown_ticks = phase_flags & 0x7fff;
        if a.character as usize >= ROSTER.len() {
            return None;
        }
        let race = crate::roster::character(a.character).race;
        if matches!(
            race,
            crate::roster::Race::Beast
                | crate::roster::Race::Spirit
                | crate::roster::Race::Dragon
                | crate::roster::Race::Fairy
                | crate::roster::Race::Witch
        ) {
            let charge = contact_ticks & 127;
            if phase_ticks > 240
                || cooldown_ticks > 480
                || charge > 36
                || (charge > 0 && (phase_ticks == 0 || contact_ticks & 128 == 0))
                || (race != crate::roster::Race::Spirit && charge > 0)
                || (race != crate::roster::Race::Beast && phase_ticks > 0 && cooldown_ticks > 0)
                || (race == crate::roster::Race::Dragon && phase_ticks > 180)
                || (crate::race_skill::has_surge(race)
                    && (phase_ticks > 9 || charge != 0 || (phase_ticks > 0 && surge_charges == 3)))
            {
                return None;
            }
            a.race_skill = crate::race_skill::State {
                remaining: phase_ticks as f32 * DT,
                cooldown: cooldown_ticks as f32 * DT,
                charge: charge as f32 * DT,
                down: phase_flags & 0x8000 != 0,
                fire_latched: contact_ticks & 128 != 0,
                surge_charges,
                surge_recharge: recharge_ticks as f32 * DT,
                ..Default::default()
            };
        } else {
            if contact_ticks > 12
                || phase_ticks > 180
                || cooldown_ticks > 720
                || (phase_ticks > 0 && cooldown_ticks > 0)
                || (phase_ticks > 0
                    && crate::roster::character(a.character).race != crate::roster::Race::Ghost)
            {
                return None;
            }
            a.ghost_phase = (contact_ticks as f32 * DT).min(crate::movement::PHASE_LINGER);
            a.phase_skill.remaining = (phase_ticks as f32 * DT).min(crate::phase_skill::DURATION);
            a.phase_skill.cooldown = (cooldown_ticks as f32 * DT).min(crate::phase_skill::COOLDOWN);
            a.phase_skill.down = phase_flags & 0x8000 != 0;
            a.race_skill.surge_charges = surge_charges;
            a.race_skill.surge_recharge = recharge_ticks as f32 * DT;
        }
        a.ability_cooldown = r.f32()?;
        if a.character == 6 {
            a.combat.chloe = crate::chloe_skill::State::unpack(a.combat.chloe.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 2 && action == 3 {
            let (state, alt_down) = crate::erpin_skill::State::unpack(r.u32()?)?;
            a.combat.erpin_skill = state;
            a.combat.erpin.alt_down = alt_down;
            a.ability_flash = 0.0;
        } else if a.character == 5 {
            a.combat.picora = crate::picora_skill::State::unpack(a.combat.picora.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 4 {
            a.combat.aya = crate::aya_skill::State::unpack(a.combat.aya.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 7 {
            a.combat.renua = crate::renua_skill::State::unpack(a.combat.renua.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 9 {
            a.combat.bella = crate::bella_skill::State::unpack(a.combat.bella.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 8 {
            a.combat.mayo = crate::mayo_skill::State::unpack(a.combat.mayo.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 10 {
            a.combat.ui = crate::ui_skill::State::unpack(a.combat.ui.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 0 {
            a.combat.vivi = crate::vivi_skill::State::unpack(a.combat.vivi.phase, r.u32()?)?;
            a.combat.mercury = a.combat.guard > 0.0;
            a.ability_flash = 0.0;
        } else if a.character == 11 {
            a.combat.butter = crate::butter_skill::State::unpack(a.combat.butter.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 12 {
            a.combat.komi = crate::komi_skill::State::unpack(a.combat.komi.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 13 {
            a.combat.hilde = crate::hilde_skill::State::unpack(a.combat.hilde.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 14 {
            a.combat.shion = crate::shion_skill::State::unpack(a.combat.shion.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else if a.character == 3 {
            a.combat.id = crate::id_skill::State::unpack(a.combat.id.phase, r.u32()?)?;
            a.ability_flash = 0.0;
        } else {
            a.ability_flash = r.f32()?;
        }
        if a.character == 2 && action != 3 && action & 3 != 0 {
            if action & 1 != 0 {
                if !(0.0..crate::erpin::EAT_SECONDS).contains(&a.ability_flash) {
                    return None;
                }
                a.combat.erpin.elapsed = a.ability_flash;
            } else {
                if !(0.0..=crate::erpin::shot_delay() * 1.25).contains(&a.ability_flash) {
                    return None;
                }
                a.combat.erpin.shot_delay = a.ability_flash;
            }
            a.ability_flash = 0.0;
        }
        if a.combat.naia.active() {
            a.combat.naia.elapsed = a.ability_flash;
            a.ability_flash = 0.0;
            let duration = if a.combat.naia.phase == crate::naia_skill::Phase::Ultimate {
                crate::naia_skill::ULTIMATE_SECONDS
            } else {
                crate::naia_skill::TOTAL_SECONDS
            };
            if !(0.0..duration).contains(&a.combat.naia.elapsed) {
                return None;
            }
        }
        a.pos = r.v3()?;
        a.yaw = r.f32()?;
        a.pitch = r.f32()?;
        let health = r.u16()?;
        a.hp = health & 255;
        a.combat.sticker = ((health >> 8) & 15) as f32 / 15.0 * crate::picora_skill::STICKER_SECONDS;
        a.combat.fashion = (health >> 12) as f32 / 15.0 * crate::picora_skill::FASHION_SECONDS;
        a.ammo = r.u8()?;
        a.reload = r.f32()?;
        a.respawn = r.f32()?;
        a.shield = r.f32()?;
        a.kills = r.u16()?;
        a.deaths = r.u16()?;
        let actor_flags = r.u8()?;
        let killer = actor_flags >> 4;
        if killer as usize > PLAYERS {
            return None;
        }
        a.killed_by = killer.checked_sub(1).map(usize::from);
        a.human = actor_flags & 1 != 0;
        a.active = actor_flags & 8 != 0;
        if !a.active && (a.human || a.hp != 0) {
            return None;
        }
        a.dancing = actor_flags & 2 != 0;
        // Clients only render the authoritative recovery state; they do not simulate healing.
        a.combat.regen = if actor_flags & 4 != 0 { 1.0 } else { 0.0 };
        let posture = r.u8()?;
        a.wave_ticks = posture & 127;
        a.crouching = posture & 128 != 0;
        if a.wave_ticks > WAVE_TICKS {
            return None;
        }
        a.shot = r.u32()?;
        a.shot_end = r.v3()?;
        a.hit = r.f32()?;
        let status = r.u16()?;
        a.hurt = (status & 127) as f32 / 120.0;
        let morph_ticks = (status >> 7) & 255;
        if morph_ticks > 180 { return None; }
        a.combat.polymorph = morph_ticks as f32 * DT;
        a.combat.rain_slow = if status & 0x8000 != 0 { DT*2.0 } else {0.0};
        a.combat.temporal = r.u8()? as f32 / 255.0;
        let frozen_ticks = r.u8()?;
        if frozen_ticks > 180 || a.hurt > 1.0 { return None; }
        a.combat.frozen = frozen_ticks as f32 * DT;
        if a.character as usize >= ROSTER.len()
            || a.hp > a.max_hp()
            || a.ammo > kit(a.character).magazine
            || a.pos.length() > 1000.0
            || !(0.0..=(a.max_hp() as f32 * 0.08 + 0.001)).contains(&a.barrier)
            || !(0.0..=crate::movement::PHASE_LINGER).contains(&a.ghost_phase)
            || !(0.0..=20.0).contains(&a.ability_cooldown)
            || !(0.0..=0.4).contains(&a.ability_flash)
        {
            return None;
        }
    }
    let len = r.u8()? as usize;
    w.event = std::str::from_utf8(r.0.get(r.1..r.1 + len)?).ok()?.into();
    r.1 += len;
    let ack = InputAck {
        slot: r.u8()? as usize,
        sequence: r.u64()?,
    };
    let local = w.actors.get_mut(ack.slot)?;
    local.vy = r.f32()?;
    if local.vy.abs() > 200.0 {
        return None;
    }
    for timer in [
        &mut local.combat.slow,
        &mut local.combat.speed_up,
        &mut local.combat.stun,
        &mut local.combat.sleep,
        &mut local.combat.dash,
    ] {
        *timer = r.u16()? as f32 * DT;
        if *timer > 60.0 {
            return None;
        }
    }
    let skill_vector = r.v3()?;
    if skill_vector.length() > 1000.0 {
        return None;
    }
    let race = crate::roster::character(local.character).race;
    if race == crate::roster::Race::Beast || crate::race_skill::has_surge(race) {
        let max_speed = if crate::race_skill::has_surge(race) {
            crate::race_skill::SURGE_SPEED
        } else {
            12.0
        };
        if skill_vector.y != 0.0 || skill_vector.length() > max_speed + 0.01 {
            return None;
        }
        local.race_skill.velocity = skill_vector;
    } else {
        local.phase_skill.safe_position = skill_vector;
    }
    r.done().then_some((seq, w, ack))
}
// Pair this sidecar with the exact movement snapshot, so packet reordering can
// never reconcile a teleported body against an old camera or portal pair.
fn portal_snapshot(world: &World, token: u64, seq: u64) -> Vec<u8> {
    let mut w = Writer::new(PORTALS);
    w.u64(token);
    w.u64(seq);
    for a in &world.actors {
        let s = &a.portals;
        for end in s.ends {
            w.u8(end.map_or(255, |p| p.face));
            if let Some(p) = end {
                w.v3(p.center);
            }
        }
        for cd in s.cooldown {
            w.u16((cd / DT).round() as u16);
        }
        w.u8(s.down[0] as u8 | (s.down[1] as u8) << 1);
        w.u8((s.lock / DT).ceil() as u8);
        w.u8((s.reopen / DT).round() as u8);
        w.v3(s.momentum);
        w.f32(s.yaw_offset);
        w.f32(s.pitch_offset);
        w.u16(s.transits);
    }
    debug_assert!(w.0.len() < MAX_PACKET);
    w.0
}

fn read_portals(bytes: &[u8], token: u64) -> Option<(u64, [crate::portal::State; PLAYERS])> {
    let mut r = Reader::new(bytes, PORTALS)?;
    if r.u64()? != token {
        return None;
    }
    let seq = r.u64()?;
    let mut states = [crate::portal::State::default(); PLAYERS];
    for s in &mut states {
        for end in &mut s.ends {
            let face = r.u8()?;
            if face == 255 {
                continue;
            }
            if face > 5 {
                return None;
            }
            let center = r.v3()?;
            if center.length() > 1000.0 {
                return None;
            }
            *end = Some(crate::portal::Portal { center, face });
        }
        for cd in &mut s.cooldown {
            let ticks = r.u16()?;
            if ticks > 300 {
                return None;
            }
            *cd = (ticks as f32 * DT).min(crate::portal::COOLDOWN);
        }
        let down = r.u8()?;
        let lock = r.u8()?;
        let reopen = r.u8()?;
        let max_lock = (crate::portal::REOPEN_DELAY / DT).round() as u8;
        if down > 3 || lock > max_lock || reopen > max_lock {
            return None;
        }
        s.down = [down & 1 != 0, down & 2 != 0];
        s.lock = lock as f32 * DT;
        s.reopen = reopen as f32 * DT;
        s.momentum = r.v3()?;
        s.yaw_offset = r.f32()?;
        s.pitch_offset = r.f32()?;
        s.transits = r.u16()?;
        if s.momentum.length() > 200.0 || s.yaw_offset.abs() > 1.0e6 || s.pitch_offset.abs() > 1.0e6
        {
            return None;
        }
    }
    r.done().then_some((seq, states))
}

fn projectile_snapshot(world: &World, token: u64, seq: u64) -> Vec<u8> {
    let mut w = Writer::new(PROJECTILES);
    w.u64(token);
    w.u64(seq);
    w.u8(world.projectiles.len() as u8);
    for p in &world.projectiles {
        w.u32(p.id);
        // Wave heading uses spare owner/kind bits; 96 projectiles still fit one MTU.
        let heading = if p.kind == ProjectileKind::MercuryField {
            (p.age * 20.0).round().clamp(0.0, 118.0) as u8
        } else if matches!(p.kind, ProjectileKind::SnowGarden | ProjectileKind::RainField | ProjectileKind::BoulderBlast | ProjectileKind::Confetti | ProjectileKind::ErpinBlast | ProjectileKind::ErpinCrash | ProjectileKind::BellaHeal) {
            (p.age * 20.0).round().clamp(0.0, 110.0) as u8
        } else if matches!(p.kind, ProjectileKind::Wave | ProjectileKind::TimeField | ProjectileKind::Butterfly | ProjectileKind::Doll | ProjectileKind::BellaFire) {
            (p.velocity
                .x
                .atan2(p.velocity.z)
                .rem_euclid(std::f32::consts::TAU)
                / std::f32::consts::TAU
                * 128.0)
                .round() as u8
                & 127
        } else {
            0
        };
        w.u8(p.owner as u8 | (heading & 15) << 4);
        w.u8(p.kind as u8 | (heading >> 4) << 5);
        for coord in [p.pos.x, p.pos.y, p.pos.z] {
            w.u16((coord * 100.0).round() as i16 as u16);
        }
    }
    debug_assert!(w.0.len() < MAX_PACKET);
    w.0
}
fn read_projectiles(bytes: &[u8], token: u64) -> Option<(u64, Vec<Projectile>)> {
    let mut r = Reader::new(bytes, PROJECTILES)?;
    if r.u64()? != token {
        return None;
    }
    let seq = r.u64()?;
    let count = r.u8()? as usize;
    if count > MAX_PROJECTILES {
        return None;
    }
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        let id = r.u32()?;
        let packed_owner = r.u8()?;
        let packed_kind = r.u8()?;
        let owner = (packed_owner & 15) as usize;
        let kind = ProjectileKind::from_wire(packed_kind & 31)?;
        let heading = (packed_owner >> 4) | (packed_kind >> 5) << 4;
        if !matches!(kind, ProjectileKind::Wave | ProjectileKind::TimeField | ProjectileKind::SnowGarden | ProjectileKind::RainField | ProjectileKind::Butterfly | ProjectileKind::Doll | ProjectileKind::BoulderBlast | ProjectileKind::Confetti | ProjectileKind::ErpinBlast | ProjectileKind::ErpinCrash | ProjectileKind::BellaFire | ProjectileKind::BellaHeal | ProjectileKind::MercuryField) && heading != 0 {
            return None;
        }
        let pos = V3::new(
            r.u16()? as i16 as f32 / 100.0,
            r.u16()? as i16 as f32 / 100.0,
            r.u16()? as i16 as f32 / 100.0,
        );
        if owner >= PLAYERS
            || pos.length() > 181.0
            || result.iter().any(|p: &Projectile| p.id == id)
        {
            return None;
        }
        let mut projectile = Projectile::visual(id, owner, kind, pos);
        if kind == ProjectileKind::MercuryField {
            if heading > 118 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = (crate::vivi_skill::FIELD_SECONDS - projectile.age).max(0.0);
        }
        if matches!(kind, ProjectileKind::Wave | ProjectileKind::TimeField | ProjectileKind::Butterfly | ProjectileKind::Doll | ProjectileKind::BellaFire) {
            projectile.velocity = direction(heading as f32 / 128.0 * std::f32::consts::TAU, 0.0)
                * crate::naia_skill::WAVE_SPEED;
        }
        if kind == ProjectileKind::TimeField { projectile.life = crate::renua_skill::FIELD_SECONDS; }
        if matches!(kind, ProjectileKind::ErpinBlast | ProjectileKind::ErpinCrash) {
            if heading > 12 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = (crate::erpin_skill::BLAST_SECONDS - projectile.age).max(0.0);
        }
        if kind == ProjectileKind::Confetti {
            if heading > 14 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = (crate::chloe_skill::CONFETTI_SECONDS-projectile.age).max(0.0);
        }
        if kind == ProjectileKind::BoulderBlast {
            if heading > 13 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = crate::butter_skill::BLAST_SECONDS - projectile.age;
        }
        if kind == ProjectileKind::BellaFire {
            projectile.velocity = projectile.velocity.normalized();
            projectile.life = crate::bella_skill::FIRE_SECONDS;
        }
        if kind == ProjectileKind::BellaHeal {
            if heading > 18 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = crate::bella_skill::WAVE_SECONDS - projectile.age;
        }
        if kind == ProjectileKind::RainField {
            if heading > 80 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = crate::ui_skill::RAIN_SECONDS - projectile.age;
        }
        if kind == ProjectileKind::SnowGarden {
            if heading > 110 { return None; }
            projectile.age = heading as f32 / 20.0;
            projectile.life = crate::aya_skill::SNOW_SECONDS - projectile.age;
        }
        result.push(projectile);
    }
    r.done().then_some((seq, result))
}
fn impact_snapshot(world: &World, token: u64, seq: u64) -> Vec<u8> {
    let mut w = Writer::new(IMPACTS);
    w.u64(token);
    w.u64(seq);
    w.u8(world.map_id as u8);
    w.u16(world.round);
    w.u64(world.tick);
    w.u8(world.impacts.len() as u8);
    for hit in &world.impacts {
        w.u32(hit.id);
        w.u64(hit.tick);
        w.v3(hit.position);
        w.u8(hit.face);
        for color in hit.color {
            w.u8(color);
        }
        w.f32(hit.radius);
        w.u8(hit.kind as u8);
        w.u8(hit.rotation);
    }
    debug_assert!(w.0.len() < MAX_PACKET);
    w.0
}

struct ImpactSnapshot {
    sequence: u64,
    map: crate::level::MapId,
    round: u16,
    impacts: Vec<Impact>,
}

fn read_impacts(bytes: &[u8], token: u64) -> Option<ImpactSnapshot> {
    let mut r = Reader::new(bytes, IMPACTS)?;
    if r.u64()? != token {
        return None;
    }
    let sequence = r.u64()?;
    let map = crate::level::MapId::from_wire(r.u8()?)?;
    let round = r.u16()?;
    let tick = r.u64()?;
    let count = r.u8()? as usize;
    if count > MAX_IMPACTS {
        return None;
    }
    let mut impacts = Vec::with_capacity(count);
    for _ in 0..count {
        let hit = Impact {
            id: r.u32()?,
            tick: r.u64()?,
            position: r.v3()?,
            face: r.u8()?,
            color: [r.u8()?, r.u8()?, r.u8()?],
            radius: r.f32()?,
            kind: ImpactKind::from_wire(r.u8()?)?,
            rotation: r.u8()?,
        };
        if hit.face >= 6
            || !hit.kind.size_range().contains(&hit.radius)
            || hit.position.length() > 181.0
            || hit.tick > tick
            || (tick - hit.tick) as f32 * DT >= IMPACT_SECONDS
            || impacts.iter().any(|old: &Impact| old.id == hit.id)
        {
            return None;
        }
        impacts.push(hit);
    }
    r.done().then_some(ImpactSnapshot {
        sequence,
        map,
        round,
        impacts,
    })
}

fn input_packet(token: u64, seq: u64, input: Input) -> Vec<u8> {
    let mut w = Writer::new(INPUT);
    w.u64(token);
    w.u64(seq);
    w.f32(input.forward);
    w.f32(input.strafe);
    w.f32(input.yaw);
    w.f32(input.pitch);
    w.u8(input.fire as u8
        | (input.jump as u8) << 1
        | (input.reload as u8) << 2
        | (input.phase_shift as u8) << 3
        | (input.ability as u8) << 4
        | (input.ultimate as u8) << 5
        | (input.dance as u8) << 6
        | (input.crouch as u8) << 7);
    w.u8(input.alt_fire as u8 | (input.wake_rest as u8) << 1);
    w.f32(input.sanitized().view_aspect);
    w.0
}
fn read_input(bytes: &[u8]) -> Option<(u64, u64, Input)> {
    let mut r = Reader::new(bytes, INPUT)?;
    let token = r.u64()?;
    let seq = r.u64()?;
    let mut input = Input {
        forward: r.f32()?,
        strafe: r.f32()?,
        yaw: r.f32()?,
        pitch: r.f32()?,
        ..Default::default()
    };
    let flags = r.u8()?;
    input.fire = flags & 1 != 0;
    input.jump = flags & 2 != 0;
    input.reload = flags & 4 != 0;
    input.phase_shift = flags & 8 != 0;
    input.ability = flags & 16 != 0;
    input.ultimate = flags & 32 != 0;
    input.dance = flags & 64 != 0;
    input.crouch = flags & 128 != 0;
    let alt = r.u8()?;
    if alt > 3 {
        return None;
    }
    input.alt_fire = alt & 1 != 0;
    input.wake_rest = alt & 2 != 0;
    input.view_aspect = r.f32()?;
    if !(0.5..=4.0).contains(&input.view_aspect) { return None; }
    r.done().then_some((token, seq, input.sanitized()))
}

fn voice_packet(kind: u8, token: u64, sequence: u64, sender: usize, samples: &Samples) -> Vec<u8> {
    let mut w = Writer::new(kind);
    w.u64(token);
    w.u64(sequence);
    if kind == VOICE_DELIVERY {
        w.u8(sender as u8);
    }
    for sample in samples {
        w.0.extend_from_slice(&sample.to_le_bytes());
    }
    w.0
}
fn read_voice(bytes: &[u8], kind: u8) -> Option<(u64, VoiceFrame)> {
    let mut r = Reader::new(bytes, kind)?;
    let token = r.u64()?;
    let sequence = r.u64()?;
    if sequence == 0 {
        return None;
    }
    let sender = if kind == VOICE_DELIVERY {
        r.u8()? as usize
    } else {
        0
    };
    if sender >= PLAYERS {
        return None;
    }
    let mut samples = [0; FRAME_SAMPLES];
    for sample in &mut samples {
        *sample = i16::from_le_bytes(r.bytes()?);
    }
    r.done().then_some((
        token,
        VoiceFrame {
            sender,
            sequence,
            samples,
        },
    ))
}

#[derive(Debug)]
struct Peer {
    identity: Option<PlayerIdentity>,
    voice_rate: VoiceRate,
    communication_sequence: u64,
    communication_result: CommunicationResult,
    addr: PeerAddr,
    nonce: u64,
    token: u64,
    slot: usize,
    sequence: u64,
    last_seen: Instant,
    input: Input,
    selection_sequence: u64,
    selection_result: SelectionResult,
    chat_sequence: u64,
    chat_outbox: VecDeque<ChatMessage>,
    last_chat: Instant,
}

#[derive(Debug)]
pub struct Server {
    gate: Option<HostGate>,
    used_tickets: HashSet<String>,
    last_roster: Instant,
    pub player_names: [String; PLAYERS],
    pub voice: VoiceInbox,
    voice_sequence: u64,
    socket: Socket,
    peers: Vec<Peer>,
    pub world: World,
    pub local: Option<usize>,
    sequence: u64,
    token_counter: u64,
    pub chat: ChatHistory,
    chat_sequence: u64,
}
impl Server {
    pub fn bind(addr: impl ToSocketAddrs, local: bool) -> io::Result<Self> {
        Self::with_socket(Socket::bind(addr)?, local)
    }
    pub fn host_eos(code: Option<&RoomCode>, local: bool) -> io::Result<Self> {
        Self::with_socket(Socket::host_eos(code)?, local)
    }
    fn with_socket(socket: Socket, local: bool) -> io::Result<Self> {
        let world = World::room(crate::level::MapId::default(), local);
        Ok(Self {
            gate: None,
            used_tickets: HashSet::new(),
            last_roster: Instant::now() - Duration::from_secs(2),
            player_names: std::array::from_fn(|i| format!("PLAYER {}", i + 1)),
            socket,
            peers: vec![],
            voice: VoiceInbox::default(),
            voice_sequence: 0,
            world,
            local: local.then_some(0),
            sequence: 0,
            token_counter: nonce(),
            chat: ChatHistory::default(),
            chat_sequence: 0,
        })
    }
    pub fn addr(&self) -> io::Result<SocketAddr> {
        self.socket.local_addr()
    }
    pub fn room_code(&self) -> Option<String> {
        self.socket.room_code()
    }
    pub fn is_eos(&self) -> bool {
        self.socket.is_eos()
    }
    pub fn ready(&self) -> bool {
        self.socket.ready()
    }
    pub fn network_label(&self) -> &'static str {
        self.socket.network_label()
    }
    pub fn restrict_to_invites(&mut self, gate: HostGate, host: PlayerIdentity) {
        self.gate = Some(gate);
        if let Some(slot) = self.local {
            self.player_names[slot] = host.display_name;
        }
    }
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
    pub fn start(&mut self) {
        self.world.start();
    }
    pub fn send_voice(&mut self, samples: Samples) {
        if let Some(slot) = self.local {
            self.broadcast_voice(slot, samples);
        }
    }
    fn broadcast_voice(&mut self, sender: usize, samples: Samples) {
        self.voice_sequence += 1;
        if self
            .local
            .is_some_and(|slot| slot != sender && team(slot) == team(sender))
        {
            self.voice.push(VoiceFrame {
                sender,
                sequence: self.voice_sequence,
                samples,
            });
        }
        for peer in &self.peers {
            if peer.slot != sender
                && team(peer.slot) == team(sender)
                && peer.last_seen.elapsed() < TIMEOUT
            {
                let packet = voice_packet(
                    VOICE_DELIVERY,
                    peer.token,
                    self.voice_sequence,
                    sender,
                    &samples,
                );
                let _ = self.socket.send_to(&packet, peer.addr);
            }
        }
    }
    pub fn send_chat(&mut self, channel: ChatChannel, text: &str) -> Result<(), &'static str> {
        let slot = self.local.ok_or("No local player")?;
        let text = clean_text(text);
        if text.is_empty() {
            return Err("Empty message");
        }
        self.broadcast_chat(slot, channel, text);
        Ok(())
    }
    pub fn communicate(&mut self, action: Communication) -> CommunicationResult {
        let Some(slot) = self.local else {
            return CommunicationResult::Unavailable;
        };
        self.communicate_as(slot, action)
    }
    fn communicate_as(&mut self, slot: usize, action: Communication) -> CommunicationResult {
        match self.world.communicate(slot, action) {
            Ok(text) => {
                self.broadcast_message(slot, ChatChannel::Team, text, action == Communication::VoiceLine);
                CommunicationResult::Accepted
            }
            Err(result) => result,
        }
    }
    fn broadcast_chat(&mut self, sender: usize, channel: ChatChannel, text: String) {
        self.broadcast_message(sender, channel, text, false);
    }
    fn broadcast_message(&mut self, sender: usize, channel: ChatChannel, text: String, voice_line: bool) {
        self.chat_sequence += 1;
        let message = ChatMessage {
            id: self.chat_sequence,
            sender,
            character: self.world.actors[sender].character,
            channel,
            text,
            voice_line,
        };
        if self
            .local
            .is_some_and(|slot| channel.visible_to(sender, slot))
        {
            self.chat.insert(message.clone());
        }
        for peer in &mut self.peers {
            if channel.visible_to(sender, peer.slot) {
                // Bound memory even when a connected peer stops acknowledging chat.
                if peer.chat_outbox.len() == CHAT_HISTORY {
                    peer.chat_outbox.pop_front();
                }
                peer.chat_outbox.push_back(message.clone());
                peer.last_chat = Instant::now() - CHAT_RETRY;
            }
        }
    }
    fn welcome(&self, p: &Peer) {
        let mut w = Writer::new(WELCOME);
        w.u64(p.nonce);
        w.u64(p.token);
        w.u8(p.slot as u8);
        let _ = self.socket.send_to(&w.0, p.addr);
    }
    pub fn update(&mut self, local_input: Input) -> io::Result<()> {
        let now = Instant::now();
        let mut buf = [0u8; MAX_PACKET + 1];
        // Bounded work per tick prevents a busy socket starving the simulation.
        for _ in 0..128 {
            let (len, addr) = match self.socket.recv_from(&mut buf) {
                Ok(v) => v,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e)
                    if e.kind() == io::ErrorKind::ConnectionReset
                        || e.raw_os_error() == Some(10040) =>
                {
                    continue
                }
                Err(e) => return Err(e),
            };
            let bytes = &buf[..len];
            let invited = bytes.get(4) == Some(&INVITED_HELLO);
            if let Some(mut r) = Reader::new(bytes, if invited { INVITED_HELLO } else { HELLO }) {
                if let Some(nonce) = r.u64() {
                    let ticket = if invited { r.invitation() } else { None };
                    if !r.done() || (invited && ticket.is_none()) {
                        continue;
                    }
                    if let Some(peer) = self.peers.iter().find(|p| p.addr == addr) {
                        if peer.nonce == nonce {
                            self.welcome(peer);
                        }
                        continue;
                    }
                    let verified = match (&self.gate, ticket.as_deref()) {
                        (Some(gate), Some(ticket)) => gate.verify(ticket, unix_now()),
                        _ => None,
                    };
                    if (invited && self.gate.is_none())
                        || (self.gate.is_some()
                            && (verified.is_none()
                                || verified.as_ref().is_some_and(|v| {
                                    self.used_tickets.contains(&v.ticket_id)
                                        || self.used_tickets.len() >= 4096
                                        || self.peers.iter().any(|p| {
                                            p.identity
                                                .as_ref()
                                                .is_some_and(|id| id.id == v.identity.id)
                                        })
                                })))
                    {
                        let _ = self.socket.send_to(&Writer::new(DENIED).0, addr);
                        continue;
                    }
                    let slot = (0..TEAM_SIZE)
                        .flat_map(|i| [i, i + TEAM_SIZE])
                        .filter(|&i| !self.world.actors[i].human)
                        .min_by_key(|&i| self.world.actors[i].active);
                    if let Some(slot) = slot {
                        self.world.join_player(slot);
                        let identity = verified.map(|v| {
                            self.used_tickets.insert(v.ticket_id);
                            v.identity
                        });
                        self.player_names[slot] = identity
                            .as_ref()
                            .map(|p| p.display_name.clone())
                            .unwrap_or_else(|| format!("PLAYER {}", slot + 1));
                        self.last_roster = now - Duration::from_secs(2);
                        self.token_counter = self.token_counter.wrapping_add(1);
                        let token = self.token_counter;
                        let peer = Peer {
                            identity,
                            voice_rate: VoiceRate::default(),
                            communication_sequence: 0,
                            communication_result: CommunicationResult::Accepted,
                            addr,
                            nonce,
                            token,
                            slot,
                            sequence: 0,
                            last_seen: now,
                            input: Input {
                                yaw: self.world.actors[slot].yaw,
                                ..Default::default()
                            },
                            selection_sequence: 0,
                            selection_result: SelectionResult::Accepted,
                            chat_sequence: 0,
                            chat_outbox: VecDeque::new(),
                            last_chat: now - CHAT_RETRY,
                        };
                        self.welcome(&peer);
                        self.peers.push(peer);
                        self.world.event = format!(
                            "Player {} joined {}",
                            slot % TEAM_SIZE + 1,
                            team_name(team(slot))
                        );
                        println!("JOIN {addr} slot={slot}");
                    } else {
                        let _ = self.socket.send_to(&Writer::new(FULL).0, addr);
                    }
                }
            } else if let Some((token, frame)) = read_voice(bytes, VOICE_SEND) {
                if let Some(peer) = self.peers.iter_mut().find(|p| {
                    p.addr == addr && p.token == token && now.duration_since(p.last_seen) < TIMEOUT
                }) {
                    if peer.voice_rate.accept(frame.sequence, now) {
                        let sender = peer.slot;
                        self.broadcast_voice(sender, frame.samples);
                    }
                }
            } else if let Some((token, seq, action)) = read_communication(bytes) {
                if let Some(index) = self
                    .peers
                    .iter()
                    .position(|p| p.addr == addr && p.token == token)
                {
                    if seq == self.peers[index].communication_sequence + 1 {
                        let slot = self.peers[index].slot;
                        let result = self.communicate_as(slot, action);
                        self.peers[index].communication_sequence = seq;
                        self.peers[index].communication_result = result;
                    }
                    let peer = &self.peers[index];
                    let mut reply = Writer::new(COMMUNICATED);
                    reply.u64(token);
                    reply.u64(peer.communication_sequence);
                    reply.u8(peer.communication_result as u8);
                    let _ = self.socket.send_to(&reply.0, addr);
                }
            } else if let Some((token, seq, channel, text)) = read_chat_request(bytes) {
                if let Some(index) = self
                    .peers
                    .iter()
                    .position(|p| p.addr == addr && p.token == token)
                {
                    // Sender and team come from the authenticated peer, never from packet data.
                    if seq == self.peers[index].chat_sequence + 1 {
                        self.peers[index].chat_sequence = seq;
                        let slot = self.peers[index].slot;
                        self.broadcast_chat(slot, channel, text);
                    }
                    let _ = self.socket.send_to(
                        &chat_ack(CHAT_ACCEPTED, token, self.peers[index].chat_sequence),
                        addr,
                    );
                }
            } else if let Some((token, id)) = read_chat_ack(bytes, CHAT_RECEIVED) {
                if let Some(peer) = self
                    .peers
                    .iter_mut()
                    .find(|p| p.addr == addr && p.token == token)
                {
                    peer.chat_outbox.retain(|message| message.id != id);
                }
            } else if let Some(mut r) = Reader::new(bytes, SELECT) {
                if let (Some(token), Some(seq), Some(id)) = (r.u64(), r.u64(), r.u8()) {
                    if !r.done() {
                        continue;
                    }
                    if let Some(p) = self
                        .peers
                        .iter_mut()
                        .find(|p| p.addr == addr && p.token == token)
                    {
                        if seq > p.selection_sequence {
                            p.selection_sequence = seq;
                            p.selection_result = self.world.select_character(p.slot, id);
                        }
                        let mut reply = Writer::new(SELECTED);
                        reply.u64(p.token);
                        reply.u64(p.selection_sequence);
                        reply.u8(p.selection_result as u8);
                        let _ = self.socket.send_to(&reply.0, addr);
                    }
                }
            } else if let Some((token, seq, input)) = read_input(bytes) {
                if let Some(p) = self
                    .peers
                    .iter_mut()
                    .find(|p| p.addr == addr && p.token == token)
                {
                    if seq > p.sequence {
                        p.sequence = seq;
                        p.input = input;
                        p.last_seen = now;
                    }
                }
            }
        }
        if now.duration_since(self.last_roster) >= Duration::from_secs(1) {
            self.last_roster = now;
            for peer in &self.peers {
                for (slot, name) in self.player_names.iter().enumerate() {
                    let mut packet = Writer::new(PLAYER_NAME);
                    packet.u64(peer.token);
                    packet.u8(slot as u8);
                    packet.text(name);
                    let _ = self.socket.send_to(&packet.0, peer.addr);
                }
            }
        }
        self.peers.retain(|p| {
            if now.duration_since(p.last_seen) > TIMEOUT {
                self.world.clear_slot(p.slot);
                self.player_names[p.slot] = format!("PLAYER {}", p.slot + 1);
                println!("LEAVE {} slot={} (empty)", p.addr, p.slot);
                false
            } else {
                true
            }
        });
        let mut inputs = [Input::default(); PLAYERS];
        if let Some(i) = self.local {
            inputs[i] = local_input;
        }
        for p in &self.peers {
            // Stop moving/shooting after a dropped connection, before clearing the slot.
            inputs[p.slot] = if now.duration_since(p.last_seen) < Duration::from_millis(300) {
                p.input
            } else {
                Input {
                    yaw: p.input.yaw,
                    pitch: p.input.pitch,
                    ..Default::default()
                }
            };
        }
        self.world.tick(inputs);
        for peer in &mut self.peers {
            if !peer.chat_outbox.is_empty() && now.duration_since(peer.last_chat) >= CHAT_RETRY {
                for message in &peer.chat_outbox {
                    let _ = self
                        .socket
                        .send_to(&chat_delivery(peer.token, message), peer.addr);
                }
                peer.last_chat = now;
            }
        }
        self.sequence = self.sequence.wrapping_add(1);
        if self.sequence.is_multiple_of(3) {
            for p in &self.peers {
                let _ = self
                    .socket
                    .send_to(&stats_snapshot(&self.world, p.token, self.sequence), p.addr);
                let _ = self.socket.send_to(
                    &portal_snapshot(&self.world, p.token, self.sequence),
                    p.addr,
                );
                let _ = self.socket.send_to(
                    &peer_snapshot(
                        &self.world,
                        p.token,
                        self.sequence,
                        InputAck {
                            slot: p.slot,
                            sequence: p.sequence,
                        },
                    ),
                    p.addr,
                );
                let _ = self.socket.send_to(
                    &projectile_snapshot(&self.world, p.token, self.sequence),
                    p.addr,
                );
                let _ = self.socket.send_to(
                    &impact_snapshot(&self.world, p.token, self.sequence),
                    p.addr,
                );
            }
        }
        Ok(())
    }
}

fn nonce() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

struct InviteTicket(String);
impl std::fmt::Debug for InviteTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted invite]")
    }
}
#[derive(Debug)]
pub struct Client {
    pub motion: ClientMotion,
    sent_inputs: VecDeque<(u64, Instant)>,
    rtt: Option<Duration>,
    invite_ticket: Option<InviteTicket>,
    pub player_names: [String; PLAYERS],
    pub voice: VoiceInbox,
    voice_sequence: u64,
    communication_sequence: u64,
    communication_outbox: VecDeque<(u64, Communication)>,
    last_communication: Instant,
    pub communication_result: Option<CommunicationResult>,
    socket: Socket,
    nonce: u64,
    token: u64,
    pub slot: Option<usize>,
    pub world: World,
    sequence: u64,
    pub received_sequence: u64,
    projectile_sequence: u64,
    impact_sequence: u64,
    stats_sequence: u64,
    match_stats: [crate::match_stats::MatchStats; PLAYERS],
    pending_worlds: VecDeque<(u64, World, InputAck)>,
    pending_portals: VecDeque<(u64, [crate::portal::State; PLAYERS])>,
    last_hello: Instant,
    last_seen: Instant,
    pub error: Option<String>,
    pub received: bool,
    pub selection_result: Option<SelectionResult>,
    pending_selection: Option<(u64, u8)>,
    selection_sequence: u64,
    last_selection: Instant,
    pub chat: ChatHistory,
    chat_outbox: VecDeque<(u64, ChatChannel, String)>,
    chat_sequence: u64,
    last_chat: Instant,
}
impl Client {
    fn commit_world(&mut self) {
        let newest = self
            .pending_worlds
            .iter()
            .filter_map(|(seq, _, _)| {
                self.pending_portals
                    .iter()
                    .any(|(p, _)| p == seq)
                    .then_some(*seq)
            })
            .max();
        let Some(seq) = newest else {
            return;
        };
        let index = self
            .pending_worlds
            .iter()
            .position(|(s, _, _)| *s == seq)
            .unwrap();
        let (_, mut world, ack) = self.pending_worlds.remove(index).unwrap();
        let states = self
            .pending_portals
            .iter()
            .find(|(s, _)| *s == seq)
            .unwrap()
            .1;
        for (actor, state) in world.actors.iter_mut().zip(states) {
            actor.portals = state;
        }
        if let Some((_, sent)) = self.sent_inputs.iter().find(|(s, _)| *s == ack.sequence) {
            let sample = sent.elapsed();
            self.rtt = Some(
                self.rtt
                    .map_or(sample, |old| old.mul_f64(0.8) + sample.mul_f64(0.2)),
            );
        }
        self.sent_inputs.retain(|(s, _)| *s > ack.sequence);
        self.motion.observe(&world, ack.slot, ack.sequence);
        world.projectiles = std::mem::take(&mut self.world.projectiles);
        if world.map_id == self.world.map_id && world.round == self.world.round {
            world.impacts = std::mem::take(&mut self.world.impacts);
        }
        for (actor, stats) in world.actors.iter_mut().zip(self.match_stats) {
            actor.stats = stats;
        }
        self.world = world;
        self.received_sequence = seq;
        self.last_seen = Instant::now();
        self.received = true;
        self.pending_worlds.retain(|(s, _, _)| *s > seq);
        self.pending_portals.retain(|(s, _)| *s > seq);
    }

    pub fn connect(addr: impl ToSocketAddrs) -> io::Result<Self> {
        Self::with_socket(Socket::connect(addr)?)
    }
    pub fn connect_eos(code: &RoomCode) -> io::Result<Self> {
        Self::with_socket(Socket::join_eos(code)?)
    }
    pub fn connect_address(address: &str) -> io::Result<Self> {
        if address.trim().starts_with("eos:") {
            Self::connect_eos(&address.parse()?)
        } else {
            Self::connect(address)
        }
    }
    pub fn network_label(&self) -> &'static str {
        self.socket.network_label()
    }
    fn with_socket(socket: Socket) -> io::Result<Self> {
        Ok(Self {
            motion: ClientMotion::default(),
            sent_inputs: VecDeque::new(),
            rtt: None,
            invite_ticket: None,
            player_names: std::array::from_fn(|i| format!("PLAYER {}", i + 1)),
            socket,
            nonce: nonce(),
            voice: VoiceInbox::default(),
            voice_sequence: 0,
            communication_sequence: 0,
            communication_outbox: VecDeque::new(),
            last_communication: Instant::now() - CHAT_RETRY,
            communication_result: None,
            token: 0,
            slot: None,
            world: World::default(),
            sequence: 0,
            received_sequence: 0,
            projectile_sequence: 0,
            impact_sequence: 0,
            stats_sequence: 0,
            match_stats: Default::default(),
            pending_worlds: VecDeque::new(),
            pending_portals: VecDeque::new(),
            last_hello: Instant::now() - Duration::from_secs(1),
            last_seen: Instant::now(),
            error: None,
            received: false,
            selection_result: None,
            pending_selection: None,
            selection_sequence: 0,
            last_selection: Instant::now(),
            chat: ChatHistory::default(),
            chat_outbox: VecDeque::new(),
            chat_sequence: 0,
            last_chat: Instant::now() - CHAT_RETRY,
        })
    }
    pub fn connect_invited(addr: impl ToSocketAddrs, ticket: String) -> io::Result<Self> {
        Self::connect(addr)?.with_invitation(ticket)
    }
    pub fn with_invitation(mut self, ticket: String) -> io::Result<Self> {
        if !ticket.is_ascii() || ticket.len() >= 512 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid invitation",
            ));
        }
        self.invite_ticket = Some(InviteTicket(ticket));
        Ok(self)
    }
    pub fn snapshot_age(&self) -> Duration {
        self.last_seen.elapsed()
    }
    /// Input-to-authoritative-snapshot round trip, including server tick wait.
    pub fn rtt(&self) -> Option<Duration> {
        self.rtt
    }
    pub fn voice_connected(&self) -> bool {
        self.received
            && self.slot.is_some()
            && self.error.is_none()
            && self.snapshot_age() < Duration::from_secs(1)
    }
    pub fn send_voice(&mut self, samples: Samples) -> io::Result<()> {
        if !self.voice_connected() {
            return Ok(());
        }
        self.voice_sequence += 1;
        send(
            &self.socket,
            &voice_packet(VOICE_SEND, self.token, self.voice_sequence, 0, &samples),
        )
    }
    pub fn select_character(&mut self, id: u8) {
        self.selection_sequence += 1;
        self.pending_selection = Some((self.selection_sequence, id));
        self.selection_result = None;
        self.last_selection = Instant::now() - Duration::from_secs(1);
    }
    pub fn selection_pending(&self) -> bool {
        self.pending_selection.is_some()
    }
    pub fn send_chat(&mut self, channel: ChatChannel, text: &str) -> Result<(), &'static str> {
        if self.slot.is_none() || self.error.is_some() {
            return Err("Chat unavailable: not connected");
        }
        if self.chat_outbox.len() >= 8 {
            return Err("Chat is busy. Please try again.");
        }
        let text = clean_text(text);
        if text.is_empty() {
            return Err("Empty message");
        }
        self.chat_sequence += 1;
        self.chat_outbox
            .push_back((self.chat_sequence, channel, text));
        if self.chat_outbox.len() == 1 {
            self.last_chat = Instant::now() - CHAT_RETRY;
        }
        Ok(())
    }
    pub fn communicate(&mut self, action: Communication) -> CommunicationResult {
        if self.slot.is_none() || self.error.is_some() {
            return CommunicationResult::Unavailable;
        }
        if self.communication_outbox.len() >= 4 {
            return CommunicationResult::Cooldown;
        }
        self.communication_sequence += 1;
        self.communication_outbox
            .push_back((self.communication_sequence, action));
        if self.communication_outbox.len() == 1 {
            self.last_communication = Instant::now() - CHAT_RETRY;
        }
        CommunicationResult::Accepted
    }
    pub fn update(&mut self, input: Input) -> io::Result<()> {
        if self.error.is_some() {
            return Ok(());
        }
        if self.slot.is_none() {
            if self.last_hello.elapsed() > Duration::from_millis(500) {
                let mut w = Writer::new(if self.invite_ticket.is_some() {
                    INVITED_HELLO
                } else {
                    HELLO
                });
                w.u64(self.nonce);
                if let Some(ticket) = &self.invite_ticket {
                    w.text(&ticket.0);
                }
                send(&self.socket, &w.0)?;
                self.last_hello = Instant::now();
            }
        } else {
            if let Some((seq, action)) = self.communication_outbox.front() {
                if self.last_communication.elapsed() >= CHAT_RETRY {
                    send(
                        &self.socket,
                        &communication_request(self.token, *seq, *action),
                    )?;
                    self.last_communication = Instant::now();
                }
            }
            if let Some((seq, channel, text)) = self.chat_outbox.front() {
                if self.last_chat.elapsed() >= CHAT_RETRY {
                    send(
                        &self.socket,
                        &chat_request(self.token, *seq, *channel, text),
                    )?;
                    self.last_chat = Instant::now();
                }
            }
            if let Some((seq, id)) = self.pending_selection {
                if self.last_selection.elapsed() >= Duration::from_millis(200) {
                    let mut packet = Writer::new(SELECT);
                    packet.u64(self.token);
                    packet.u64(seq);
                    packet.u8(id);
                    send(&self.socket, &packet.0)?;
                    self.last_selection = Instant::now();
                }
            }
            self.sequence += 1;
            send(
                &self.socket,
                &input_packet(self.token, self.sequence, input.sanitized()),
            )?;
            self.sent_inputs.push_back((self.sequence, Instant::now()));
            while self.sent_inputs.len() > 120 {
                self.sent_inputs.pop_front();
            }
            if self.received {
                self.motion.advance(&self.world, self.sequence, input);
            }
        }
        let mut buf = [0u8; MAX_PACKET + 1];
        for _ in 0..64 {
            let len = match self.socket.recv(&mut buf) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::ConnectionReset => break,
                Err(e) if e.raw_os_error() == Some(10040) => continue,
                Err(e) => return Err(e),
            };
            let bytes = &buf[..len];
            if let Some(mut r) = Reader::new(bytes, PLAYER_NAME) {
                if let (Some(token), Some(slot), Some(name)) = (r.u64(), r.u8(), r.text()) {
                    if self.slot.is_some()
                        && token == self.token
                        && usize::from(slot) < PLAYERS
                        && r.done()
                        && name.chars().count() <= 32
                    {
                        self.player_names[usize::from(slot)] = name;
                    }
                }
            } else if let Some(mut r) = Reader::new(bytes, WELCOME) {
                if let (Some(nonce), Some(token), Some(slot)) = (r.u64(), r.u64(), r.u8()) {
                    if nonce == self.nonce && slot < PLAYERS as u8 && r.done() {
                        self.token = token;
                        self.slot = Some(slot as usize);
                    }
                }
            } else if let Some(mut r) = Reader::new(bytes, SELECTED) {
                if let (Some(token), Some(seq), Some(result)) = (r.u64(), r.u64(), r.u8()) {
                    if token == self.token
                        && r.done()
                        && self
                            .pending_selection
                            .is_some_and(|(pending, _)| pending == seq)
                    {
                        if let Some(result) = SelectionResult::from_wire(result) {
                            self.selection_result = Some(result);
                            self.pending_selection = None;
                        }
                    }
                }
            } else if let Some(mut r) = Reader::new(bytes, COMMUNICATED) {
                if let (Some(token), Some(seq), Some(result)) = (r.u64(), r.u64(), r.u8()) {
                    if token == self.token
                        && r.done()
                        && self
                            .communication_outbox
                            .front()
                            .is_some_and(|(id, _)| *id == seq)
                    {
                        if let Some(result) = CommunicationResult::from_wire(result) {
                            self.communication_result = Some(result);
                            self.communication_outbox.pop_front();
                            self.last_communication = Instant::now() - CHAT_RETRY;
                        }
                    }
                }
            } else if let Some((token, seq)) = read_chat_ack(bytes, CHAT_ACCEPTED) {
                if token == self.token
                    && self
                        .chat_outbox
                        .front()
                        .is_some_and(|(id, _, _)| *id == seq)
                {
                    self.chat_outbox.pop_front();
                    self.last_chat = Instant::now() - CHAT_RETRY;
                }
            } else if self.slot.is_some() {
                if let Some((token, frame)) = read_voice(bytes, VOICE_DELIVERY) {
                    if token == self.token
                        && self.voice_connected()
                        && self.slot.is_some_and(|slot| {
                            slot != frame.sender && team(slot) == team(frame.sender)
                        })
                    {
                        self.voice.push(frame);
                    }
                    continue;
                }
                if let Some(message) = read_chat_delivery(bytes, self.token) {
                    send(
                        &self.socket,
                        &chat_ack(CHAT_RECEIVED, self.token, message.id),
                    )?;
                    self.chat.insert(message);
                    continue;
                }
                if let Some((seq, world, ack)) = read_snapshot(bytes, self.token) {
                    if seq > self.received_sequence
                        && Some(ack.slot) == self.slot
                        && ack.sequence <= self.sequence
                    {
                        self.pending_worlds.push_back((seq, world, ack));
                        while self.pending_worlds.len() > 4 {
                            self.pending_worlds.pop_front();
                        }
                    }
                } else if let Some((seq, states)) = read_portals(bytes, self.token) {
                    if seq > self.received_sequence {
                        self.pending_portals.push_back((seq, states));
                        while self.pending_portals.len() > 4 {
                            self.pending_portals.pop_front();
                        }
                    }
                } else if let Some((seq, stats)) = read_stats(bytes, self.token) {
                    if seq > self.stats_sequence {
                        self.match_stats = stats;
                        self.stats_sequence = seq;
                        for (actor, stats) in self.world.actors.iter_mut().zip(stats) {
                            actor.stats = stats;
                        }
                    }
                } else if let Some((seq, projectiles)) = read_projectiles(bytes, self.token) {
                    if seq > self.projectile_sequence {
                        self.world.projectiles = projectiles;
                        self.projectile_sequence = seq;
                    }
                } else if let Some(batch) = read_impacts(bytes, self.token) {
                    if batch.sequence > self.impact_sequence
                        && batch.map == self.world.map_id
                        && batch.round == self.world.round
                    {
                        self.world.impacts = batch.impacts;
                        self.impact_sequence = batch.sequence;
                    }
                }
            } else if Reader::new(bytes, DENIED).is_some() && self.slot.is_none() {
                self.error = Some("Invitation expired, already used, or not permitted. Download a new invitation.".into());
            } else if Reader::new(bytes, FULL).is_some() {
                self.error = Some("Server is full (12 players)".into());
            }
        }
        self.commit_world();
        let timeout = if self.socket.is_eos() && !self.received {
            Duration::from_secs(75)
        } else {
            TIMEOUT
        };
        if self.last_seen.elapsed() > timeout {
            self.error = Some(if self.socket.is_eos() {
                "EOS connection timed out. Check the room code, host, Internet connection and EOS client policy."
            } else { "Connection timed out. Check server address / UDP 28080." }.into());
        }
        Ok(())
    }
}

fn send(socket: &Socket, bytes: &[u8]) -> io::Result<()> {
    match socket.send(bytes) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn eos_frames_preserve_full_game_snapshots_and_maximum_projectiles() {
        use crate::transport::{frames, Reassembler, EOS_FRAME};
        let world = World {
            projectiles: (0..MAX_PROJECTILES)
                .map(|i| {
                    Projectile::visual(
                        i as u32,
                        i % PLAYERS,
                        ProjectileKind::Boulder,
                        V3::new(-12.3, 2.0, 34.5),
                    )
                })
                .collect(),
            ..Default::default()
        };
        for (kind, packet) in [
            (SNAPSHOT, snapshot(&world, 44, 99)),
            (PROJECTILES, projectile_snapshot(&world, 44, 99)),
        ] {
            let mut reassembly = Reassembler::default();
            let mut received = None;
            let fragments = frames(&packet, 8).unwrap();
            assert!(fragments.iter().all(|f| f.len() <= EOS_FRAME));
            for part in fragments.into_iter().rev() {
                if let Some(complete) = reassembly.receive([1; 32], &part, Instant::now()) {
                    received = Some(complete);
                }
            }
            let received = received.unwrap();
            assert_eq!(received, packet);
            if kind == SNAPSHOT {
                assert!(read_snapshot(&received, 44).is_some());
            } else {
                assert_eq!(
                    read_projectiles(&received, 44).unwrap().1.len(),
                    MAX_PROJECTILES
                );
            }
        }
    }
    #[test]
    fn invited_host_rejects_plain_forged_expired_and_replayed_joins() {
        use crate::online::{
            tests::{gate, ticket, USER},
            unix_now, PlayerIdentity,
        };
        let now = unix_now();
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        server.restrict_to_invites(
            gate(now),
            PlayerIdentity {
                id: "host".into(),
                display_name: "호스트".into(),
            },
        );
        let address = server.addr().unwrap();
        let mut denied = [
            Client::connect(address).unwrap(),
            Client::connect_invited(address, "forged".into()).unwrap(),
            Client::connect_invited(address, ticket(now - 1, "만료")).unwrap(),
        ];
        pump(&mut server, &mut denied, 8);
        assert_eq!(server.peer_count(), 0);
        assert!(denied
            .iter()
            .all(|c| c.error.as_ref().is_some_and(|e| e.contains("Invitation"))));
        let valid = ticket(now + 300, "닷지 친구");
        let mut friends = [Client::connect_invited(address, valid.clone()).unwrap()];
        pump(&mut server, &mut friends, 8);
        assert_eq!(server.peer_count(), 1);
        let slot = friends[0].slot.unwrap();
        assert_eq!(server.peers[0].identity.as_ref().unwrap().id, USER);
        assert_eq!(friends[0].player_names[slot], "닷지 친구");
        assert_eq!(friends[0].player_names[0], "호스트");
        let mut replay = [Client::connect_invited(address, valid.clone()).unwrap()];
        pump(&mut server, &mut replay, 8);
        assert_eq!(server.peer_count(), 1);
        assert!(replay[0].error.is_some());
        server.peers[0].last_seen = Instant::now() - TIMEOUT - Duration::from_secs(1);
        server.update(Input::default()).unwrap();
        let mut reused = [Client::connect_invited(address, valid).unwrap()];
        pump(&mut server, &mut reused, 8);
        assert_eq!(server.peer_count(), 0);
        assert!(reused[0].error.is_some());
    }
    use super::*;
    #[test]
    fn selected_map_reaches_existing_and_late_clients_and_survives_start() {
        use crate::level::MapId;
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients = vec![Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        assert!(server.world.select_map(MapId::ElfCity));
        pump(&mut server, &mut clients, 12);
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 12);
        for c in &clients {
            assert_eq!(c.world.map_id, MapId::ElfCity);
            assert_eq!(c.world.map.len(), MapId::ElfCity.arena().len());
            assert_eq!(c.world.spawn_center(0), server.world.spawn_center(0));
        }
        server.start();
        pump(&mut server, &mut clients, 12);
        for c in &clients {
            assert_eq!(c.world.map_id, MapId::ElfCity);
            assert_ne!(c.world.phase, Phase::Lobby);
        }
        server.world.phase = Phase::Finished;
        assert!(server.world.select_map(MapId::FairyKingdomCastle));
        pump(&mut server, &mut clients, 12);
        for c in &clients {
            assert_eq!(c.world.map_id, MapId::FairyKingdomCastle);
        }
        let mut packet = snapshot(&server.world, 42, 1);
        packet[29] = 255;
        assert!(read_snapshot(&packet, 42).is_none());
        let mut old = snapshot(&server.world, 42, 1);
        old[..4].copy_from_slice(b"TSBC");
        assert!(read_snapshot(&old, 42).is_none());
    }
    #[test]
    fn deathcam_snapshot_reaches_victim_observer_and_clears_on_respawn() {
        use crate::{combat::AttackClass, deathcam::DeathCamera};
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients: Vec<_> = (0..2)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 15);
        let victim = clients[0].slot.unwrap();
        server.start();
        server.world.phase = Phase::Playing;
        for a in server.world.actors.iter_mut().filter(|a| a.active) {
            a.human = true;
        }
        let mut camera = DeathCamera::default();
        for _ in 0..65 {
            server.world.actors[0].pos.x += 0.02;
            pump(&mut server, &mut clients, 3);
            camera.update(&clients[0].world, victim, 3.0 * DT);
        }
        server.world.actors[victim].shield = 0.0;
        server.world.actors[victim].pos = crate::game::POINTS[0];
        server
            .world
            .damage_hit(0, victim, 1000.0, AttackClass::Basic, 3);
        pump(&mut server, &mut clients, 12);
        for client in &clients {
            assert_eq!(client.world.actors[victim].killed_by, Some(0));
            assert_eq!(
                DeathCamera::default()
                    .update(&client.world, victim, DT)
                    .unwrap()
                    .target,
                victim
            );
        }
        camera.update(&clients[0].world, victim, DT);
        assert_eq!(camera.stage, crate::deathcam::Stage::TopDown);
        let live_killer = clients[0].world.actors[0].pos;
        camera.update(
            &clients[0].world,
            victim,
            camera.death_duration() + crate::deathcam::DEATH_HOLD + 0.01,
        );
        assert_eq!(camera.stage, crate::deathcam::Stage::Replay);
        assert!(camera.scene().unwrap().actors[victim].alive());
        assert!((camera.scene().unwrap().actors[0].pos - live_killer).length() > 0.5);
        assert_eq!(camera.view.unwrap().target, 0);
        server.world.actors[victim].respawn = DT;
        pump(&mut server, &mut clients, 12);
        assert!(clients
            .iter()
            .all(|c| c.world.actors[victim].killed_by.is_none()));
        assert!(camera.update(&clients[0].world, victim, DT).is_none());
    }

    #[test]
    fn snapshot_rejects_invalid_killer_and_previous_protocol() {
        let mut world = World::default();
        world.actors[0].killed_by = Some(PLAYERS);
        assert!(read_snapshot(&snapshot(&world, 42, 1), 42).is_none());
        world.actors[0].killed_by = Some(11);
        let mut packet = snapshot(&world, 42, 1);
        assert_eq!(
            read_snapshot(&packet, 42).unwrap().1.actors[0].killed_by,
            Some(11)
        );
        packet[..4].copy_from_slice(b"TSBG");
        assert!(read_snapshot(&packet, 42).is_none());
    }
    #[test]
    fn quick_communication_is_authoritative_team_only_and_wave_reaches_opponents() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients: Vec<_> = (0..2)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 15);
        assert_eq!(clients[0].slot, Some(6));
        assert_eq!(clients[1].slot, Some(1));
        server.world.actors[1].combat.ultimate = 42.8;
        clients[1].world.actors[1].combat.ultimate = 100.0; // Client cannot forge charge.
        clients[1].communicate(Communication::UltimateStatus);
        pump(&mut server, &mut clients, 10);
        assert!(server.chat.messages[0].text.contains("42%"));
        assert_eq!(clients[1].chat.messages.len(), 1);
        assert!(clients[0].chat.messages.is_empty());
        pump(&mut server, &mut clients, 46);
        clients[1].communicate(Communication::Hello);
        pump(&mut server, &mut clients, 10);
        assert!(server.world.actors[1].wave_ticks > 0);
        assert!(clients.iter().all(|c| c.world.actors[1].wave_ticks > 0));
        assert_eq!(server.chat.messages.back().unwrap().text, "안녕하세요!");
        assert!(clients[0].chat.messages.is_empty());
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 10);
        assert!(clients[2].world.actors[1].wave_ticks > 0);
        assert!(clients[2].chat.messages.is_empty());
        assert_eq!(
            server.communicate(Communication::VoiceLine),
            CommunicationResult::Accepted
        );
        assert_eq!(server.chat.messages.len(), 3);
        assert!(server.chat.messages.back().unwrap().voice_line);
        pump(&mut server, &mut clients, 8);
        assert!(clients[1].chat.messages.back().unwrap().voice_line);
        assert!(clients[0].chat.messages.is_empty());
        server.world.actors[0].communication_cooldown = 0.0;
        assert_eq!(
            server.communicate(Communication::Emote),
            CommunicationResult::Accepted
        );
        pump(&mut server, &mut clients, 8);
        assert!(clients.iter().all(|c| c.world.actors[0].dancing));
        assert_eq!(server.chat.messages.len(), 4);
    }
    #[test]
    fn communication_retry_does_not_replay_gesture_and_rejects_bad_packets() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients = [Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        let client = &mut clients[0];
        client.communicate(Communication::Hello);
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        let slot = client.slot.unwrap();
        let original = server.world.actors[slot].wave_ticks;
        let mut buffer = [0; MAX_PACKET + 1];
        while client.socket.recv(&mut buffer).is_ok() {} // Lose action acknowledgement.
        client.last_communication = Instant::now() - CHAT_RETRY;
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        client.update(Input::default()).unwrap();
        assert_eq!(server.world.actors[slot].wave_ticks, original - 1);
        assert!(client.communication_outbox.is_empty());
        assert_eq!(
            client.communication_result,
            Some(CommunicationResult::Accepted)
        );
        let packet = communication_request(client.token, 2, Communication::Thanks);
        for n in 0..packet.len() {
            assert!(read_communication(&packet[..n]).is_none());
        }
        let mut invalid = packet.clone();
        invalid[21] = 255;
        assert!(read_communication(&invalid).is_none());
        let mut invalid = packet;
        invalid.push(0);
        assert!(read_communication(&invalid).is_none());
        client
            .socket
            .send(&communication_request(
                client.token + 1,
                2,
                Communication::Emote,
            ))
            .unwrap();
        let raw = UdpSocket::bind("127.0.0.1:0").unwrap();
        raw.send_to(
            &communication_request(client.token, 2, Communication::Emote),
            server.addr().unwrap(),
        )
        .unwrap();
        server.update(Input::default()).unwrap();
        assert!(!server.world.actors[slot].dancing);
        client.communicate(Communication::Thanks);
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        client.update(Input::default()).unwrap();
        assert_eq!(
            client.communication_result,
            Some(CommunicationResult::Cooldown)
        );
    }
    #[test]
    fn chat_packets_validate_unicode_bounds_identity_and_truncation() {
        let text = "가".repeat(160);
        let request = chat_request(42, 1, ChatChannel::Team, &text);
        assert!(request.len() < MAX_PACKET);
        assert_eq!(read_chat_request(&request).unwrap().3, text);
        for n in 0..request.len() {
            assert!(read_chat_request(&request[..n]).is_none());
        }
        let mut invalid = request.clone();
        invalid[21] = 2;
        assert!(read_chat_request(&invalid).is_none());
        invalid = request.clone();
        invalid.push(0);
        assert!(read_chat_request(&invalid).is_none());
        invalid = request;
        *invalid.last_mut().unwrap() = 255;
        assert!(read_chat_request(&invalid).is_none());
        for value in [" ".to_owned(), "x\ny".into(), "x".repeat(161)] {
            assert!(read_chat_request(&chat_request(42, 1, ChatChannel::Team, &value)).is_none());
        }
        let message = ChatMessage {
            id: 1,
            sender: 2,
            character: 3,
            channel: ChatChannel::Match,
            text: "안녕 😀".into(),
            voice_line: false,
        };
        let packet = chat_delivery(42, &message);
        assert_eq!(read_chat_delivery(&packet, 42), Some(message));
        assert!(read_chat_delivery(&packet, 43).is_none());
        for n in 0..packet.len() {
            assert!(read_chat_delivery(&packet[..n], 42).is_none());
        }
        let mut invalid = packet;
        invalid[21] = PLAYERS as u8;
        assert!(read_chat_delivery(&invalid, 42).is_none());
    }

    #[test]
    fn character_voice_delivery_is_marked_by_authority_not_typed_chat() {
        let message = ChatMessage {
            id: 1, sender: 0, character: 2, channel: ChatChannel::Team,
            text: "음성 대사".into(), voice_line: true,
        };
        let packet = chat_delivery(42, &message);
        assert_eq!(read_chat_delivery(&packet, 42), Some(message));
        let mut invalid = packet.clone();
        *invalid.last_mut().unwrap() = 2;
        assert!(read_chat_delivery(&invalid, 42).is_none());
        let mut request = chat_request(42, 1, ChatChannel::Team, "음성 대사");
        request.push(1);
        assert!(read_chat_request(&request).is_none());
    }

    #[test]
    fn udp_chat_routes_team_and_match_for_host_and_clients() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients: Vec<_> = (0..3)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 20);
        assert_eq!(
            clients.iter().map(|c| c.slot.unwrap()).collect::<Vec<_>>(),
            vec![6, 1, 7]
        );
        clients[0]
            .send_chat(ChatChannel::Team, "상대 팀 비밀")
            .unwrap();
        pump(&mut server, &mut clients, 10);
        assert!(server.chat.messages.is_empty());
        assert!(clients[1].chat.messages.is_empty());
        assert_eq!(clients[0].chat.messages.len(), 1);
        assert_eq!(clients[2].chat.messages[0].text, "상대 팀 비밀");
        server.send_chat(ChatChannel::Team, "우리 팀 집결").unwrap();
        pump(&mut server, &mut clients, 10);
        assert_eq!(server.chat.messages.len(), 1);
        assert_eq!(clients[1].chat.messages[0].sender, 0);
        assert_eq!(clients[0].chat.messages.len(), 1);
        clients[1]
            .send_chat(ChatChannel::Match, "모두 안녕하세요")
            .unwrap();
        clients[1]
            .send_chat(ChatChannel::Match, "두 번째 메시지")
            .unwrap();
        pump(&mut server, &mut clients, 20);
        assert_eq!(server.chat.messages.len(), 3);
        for client in &clients {
            assert_eq!(client.chat.messages.len(), 3);
            assert_eq!(client.chat.messages.back().unwrap().text, "두 번째 메시지");
        }
        // A restart does not reset message IDs or leak team history to a late joiner.
        server.start();
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 10);
        assert!(clients[3].chat.messages.is_empty());
        server.send_chat(ChatChannel::Match, "새 경기").unwrap();
        pump(&mut server, &mut clients, 10);
        assert!(clients
            .iter()
            .all(|c| c.chat.messages.back().unwrap().text == "새 경기"));
    }

    #[test]
    fn udp_chat_retries_lost_requests_and_acks_without_duplicates_or_spoofing() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients = [Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        let client = &mut clients[0];
        client.send_chat(ChatChannel::Match, "재전송 확인").unwrap();
        client.update(Input::default()).unwrap();
        // Drop request and movement packets before the server processes them.
        let mut buf = [0; MAX_PACKET + 1];
        while server.socket.recv_from(&mut buf).is_ok() {}
        client.last_chat = Instant::now() - CHAT_RETRY;
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        assert_eq!(server.chat.messages.len(), 1);
        // Drop acceptance and first delivery; retry both sides.
        while client.socket.recv(&mut buf).is_ok() {}
        client.last_chat = Instant::now() - CHAT_RETRY;
        server.peers[0].last_chat = Instant::now() - CHAT_RETRY;
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        client.update(Input::default()).unwrap();
        assert_eq!(client.chat.messages.len(), 1);
        assert!(client.chat_outbox.is_empty());
        assert_eq!(server.chat.messages.len(), 1);
        // Lose the delivery acknowledgement too.
        while server.socket.recv_from(&mut buf).is_ok() {}
        server.peers[0].last_chat = Instant::now() - CHAT_RETRY;
        server.update(Input::default()).unwrap();
        client.update(Input::default()).unwrap();
        server.update(Input::default()).unwrap();
        assert_eq!(client.chat.messages.len(), 1);
        assert!(server.peers[0].chat_outbox.is_empty());
        let forged = chat_request(client.token + 1, 2, ChatChannel::Match, "forged");
        client.socket.send(&forged).unwrap();
        let raw = UdpSocket::bind("127.0.0.1:0").unwrap();
        raw.send_to(
            &chat_request(client.token, 2, ChatChannel::Match, "wrong address"),
            server.addr().unwrap(),
        )
        .unwrap();
        client
            .socket
            .send(&chat_request(client.token, 1, ChatChannel::Match, "replay"))
            .unwrap();
        server.update(Input::default()).unwrap();
        assert_eq!(server.chat.messages.len(), 1);
    }

    fn pump(server: &mut Server, clients: &mut [Client], ticks: usize) {
        for _ in 0..ticks {
            for client in &mut *clients {
                client.update(Input::default()).unwrap();
            }
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn voice_packets_validate_size_sequence_sender_and_pcm() {
        let samples = std::array::from_fn(|i| (i as i16 - 160) * 120);
        for kind in [VOICE_SEND, VOICE_DELIVERY] {
            let packet = voice_packet(kind, 42, 1, 6, &samples);
            assert!(packet.len() < MAX_PACKET);
            let (token, frame) = read_voice(&packet, kind).unwrap();
            assert_eq!(token, 42);
            assert_eq!(frame.samples, samples);
            assert_eq!(frame.sender, if kind == VOICE_SEND { 0 } else { 6 });
            for length in 0..packet.len() {
                assert!(read_voice(&packet[..length], kind).is_none());
            }
            let mut extra = packet.clone();
            extra.push(0);
            assert!(read_voice(&extra, kind).is_none());
            assert!(read_voice(&voice_packet(kind, 42, 0, 6, &samples), kind).is_none());
        }
        assert!(read_voice(
            &voice_packet(VOICE_DELIVERY, 42, 1, PLAYERS, &samples),
            VOICE_DELIVERY
        )
        .is_none());
    }
    #[test]
    fn udp_voice_routes_both_directions_host_and_dedicated_without_echo_or_enemies() {
        for local in [true, false] {
            let mut server = Server::bind("127.0.0.1:0", local).unwrap();
            let mut clients: Vec<_> = (0..4)
                .map(|_| Client::connect(server.addr().unwrap()).unwrap())
                .collect();
            pump(&mut server, &mut clients, 20);
            for sender in 0..clients.len() {
                let slot = clients[sender].slot.unwrap();
                let samples = [1000 + sender as i16; FRAME_SAMPLES];
                clients[sender].send_voice(samples).unwrap();
                pump(&mut server, &mut clients, 5);
                for client in &mut clients {
                    let receiver = client.slot.unwrap();
                    let frames = client.voice.drain();
                    if receiver != slot && team(receiver) == team(slot) {
                        assert_eq!(frames.len(), 1);
                        assert_eq!(frames[0].sender, slot);
                        assert_eq!(frames[0].samples, samples);
                    } else {
                        assert!(frames.is_empty());
                    }
                }
                let received = server.voice.drain();
                assert_eq!(received.len(), usize::from(local && team(slot) == 0));
            }
            server.send_voice([2000; FRAME_SAMPLES]);
            pump(&mut server, &mut clients, 5);
            assert!(server.voice.drain().is_empty());
            for client in &mut clients {
                let frames = client.voice.drain();
                assert_eq!(
                    frames.len(),
                    usize::from(local && team(client.slot.unwrap()) == 0)
                );
            }
        }
    }
    #[test]
    fn udp_voice_rejects_spoof_replay_flood_and_handles_slot_reuse() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients: Vec<_> = (0..2)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 15);
        let sender = &clients[1];
        assert_eq!(sender.slot, Some(1));
        let outsider = UdpSocket::bind("127.0.0.1:0").unwrap();
        let packet = voice_packet(VOICE_SEND, sender.token, 1, 0, &[1000; FRAME_SAMPLES]);
        outsider.send_to(&packet, server.addr().unwrap()).unwrap();
        sender
            .socket
            .send(&voice_packet(
                VOICE_SEND,
                sender.token + 99,
                1,
                0,
                &[1000; FRAME_SAMPLES],
            ))
            .unwrap();
        pump(&mut server, &mut clients, 3);
        assert!(server.voice.drain().is_empty());
        clients[1].socket.send(&packet).unwrap();
        clients[1].socket.send(&packet).unwrap();
        pump(&mut server, &mut clients, 3);
        let first = server.voice.drain();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].sender, 1);
        let now = Instant::now();
        server
            .peers
            .iter_mut()
            .find(|p| p.slot == 1)
            .unwrap()
            .voice_rate = VoiceRate::default();
        for sequence in 2..=120 {
            clients[1]
                .socket
                .send(&voice_packet(
                    VOICE_SEND,
                    clients[1].token,
                    sequence,
                    0,
                    &[1000; FRAME_SAMPLES],
                ))
                .unwrap();
        }
        server.update(Input::default()).unwrap();
        assert!(
            server.voice_sequence
                <= first[0].sequence + 10 + (now.elapsed().as_secs_f32() * 50.0) as u64
        );
        assert!(server.voice.drain().len() <= crate::voice::QUEUED_FRAMES);
        // Old packets cannot keep a disconnected peer alive. Reusing its slot has a new token,
        // while server-assigned delivery sequences keep the listener's replay filter valid.
        let peer = server.peers.iter_mut().find(|p| p.slot == 1).unwrap();
        peer.last_seen = Instant::now() - TIMEOUT - Duration::from_secs(1);
        clients[1].socket.send(&packet).unwrap();
        server.update(Input::default()).unwrap();
        assert!(server.voice.drain().is_empty());
        clients.pop();
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 15);
        assert_eq!(clients[1].slot, Some(1));
        clients[1].send_voice([2345; FRAME_SAMPLES]).unwrap();
        pump(&mut server, &mut clients, 3);
        let frames = server.voice.drain();
        assert_eq!(frames.len(), 1);
        assert!(frames[0].sequence > first[0].sequence);
        assert_eq!(frames[0].samples[0], 2345);
    }
    #[test]
    fn udp_live_spawn_swap_preserves_charge_and_rejects_stale_zone_request() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        let mut clients: Vec<_> = (0..2)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 20);
        server.world.phase = Phase::Playing;
        for actor in server.world.actors.iter_mut().filter(|a| a.active) {
            actor.human = true;
        }
        server.world.actors[0].combat.ultimate = 83.0;
        clients[0].select_character(4);
        pump(&mut server, &mut clients, 12);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Accepted));
        for client in &clients {
            assert_eq!(client.world.actors[0].character, 4);
            assert_eq!(client.world.actors[0].combat.ultimate, 15.0);
        }
        // Client still has the previous spawn snapshot; authority uses the new position.
        server.world.actors[0].pos = V3::default();
        server.world.actors[0].combat.ultimate = 61.0;
        clients[0].select_character(7);
        pump(&mut server, &mut clients, 12);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Locked));
        assert_eq!(server.world.actors[0].character, 4);
        assert!(server.world.actors[0].combat.ultimate >= 61.0);
        server.world.point = 1;
        server.world.actors[0].pos = server.world.spawn_center(0);
        server.world.actors[0].combat.ultimate = 8.0;
        clients[0].select_character(7);
        pump(&mut server, &mut clients, 12);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Accepted));
        assert!(clients
            .iter()
            .all(|c| c.world.actors[0].character == 7 && c.world.actors[0].combat.ultimate == 8.0));
    }
    #[test]
    fn twelve_remote_clients_receive_authoritative_selection_results() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        let mut clients: Vec<_> = (0..PLAYERS)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        pump(&mut server, &mut clients, 20);
        assert_eq!(server.peer_count(), 12);
        assert!(clients.iter().all(|c| c.received && c.error.is_none()));
        assert_eq!(clients[0].slot, Some(0));
        assert_eq!(clients[1].slot, Some(6));
        assert_eq!(clients[2].slot, Some(1));
        clients[0].select_character(4);
        clients[1].select_character(4); // opposite team mirrors are legal
        clients[2].select_character(4); // same-team race: first request wins
        pump(&mut server, &mut clients, 12);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Accepted));
        assert_eq!(clients[1].selection_result, Some(SelectionResult::Accepted));
        assert_eq!(clients[2].selection_result, Some(SelectionResult::Taken));
        assert!(clients
            .iter()
            .all(|c| c.world.actors[0].character == 4 && c.world.actors[6].character == 4));
        assert_eq!(server.world.damage_bonus(0), 40);
        assert!(clients.iter().all(|c| c.world.damage_bonus(0) == 40));
        clients[0].select_character(255);
        pump(&mut server, &mut clients, 8);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Invalid));
        server.start();
        server.world.phase = Phase::Playing;
        server.world.actors[0].pos = V3::default();
        clients[0].select_character(7);
        pump(&mut server, &mut clients, 8);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Locked));
        server.world.actors[0].hp = 0;
        server.world.actors[0].respawn = 6.0;
        clients[0].select_character(7);
        pump(&mut server, &mut clients, 8);
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Accepted));
        assert!(clients
            .iter()
            .all(|c| c.world.actors[0].character == 7 && c.world.actors[0].hp == 0));
    }
    #[test]
    fn selection_retries_lost_ack_and_rejects_replay_bad_token_and_position_overflow() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        // This fixture specifically exercises exchanging heroes with a full bot team.
        server.world = World::default();
        let mut clients = [Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        // Move a middle bot into a third front position before the client request.
        assert_eq!(
            server.world.select_character(2, 3),
            SelectionResult::Accepted
        );
        clients[0].select_character(10); // exchange with Ui bot
        pump(&mut server, &mut clients, 8);
        assert_eq!(server.world.actors[0].character, 10);
        clients[0].select_character(6); // would become fourth front
        pump(&mut server, &mut clients, 8);
        assert_eq!(
            clients[0].selection_result,
            Some(SelectionResult::PositionFull)
        );
        clients[0].select_character(4);
        clients[0].update(Input::default()).unwrap();
        std::thread::sleep(Duration::from_millis(1));
        server.update(Input::default()).unwrap();
        std::thread::sleep(Duration::from_millis(1));
        // Deliberately discard all received datagrams, including SELECTED.
        let mut buf = [0u8; MAX_PACKET + 1];
        while clients[0].socket.recv(&mut buf).is_ok() {}
        assert!(clients[0].selection_pending());
        clients[0].last_selection = Instant::now() - Duration::from_secs(1);
        pump(&mut server, &mut clients, 8);
        assert!(!clients[0].selection_pending());
        assert_eq!(clients[0].selection_result, Some(SelectionResult::Accepted));
        assert_eq!(server.world.actors[0].character, 4);
        for (token, seq, id) in [(clients[0].token, 1, 0), (clients[0].token + 99, 999, 0)] {
            let mut packet = Writer::new(SELECT);
            packet.u64(token);
            packet.u64(seq);
            packet.u8(id);
            clients[0].socket.send(&packet.0).unwrap();
        }
        pump(&mut server, &mut clients, 8);
        assert_eq!(server.world.actors[0].character, 4);
        assert_eq!(server.world.position_counts(0), [3, 1, 2]);
    }
    #[test]
    fn capacity_replay_rejection_and_disconnect_leaves_empty_slot() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients: Vec<_> = (0..PLAYERS)
            .map(|_| Client::connect(server.addr().unwrap()).unwrap())
            .collect();
        for _ in 0..30 {
            for client in &mut clients {
                client.update(Input::default()).unwrap();
            }
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(server.peer_count(), PLAYERS - 1);
        assert_eq!(
            clients.iter().filter(|c| c.slot.is_some()).count(),
            PLAYERS - 1
        );
        assert_eq!(
            clients
                .iter()
                .filter(|c| c.error.as_deref() == Some("Server is full (12 players)"))
                .count(),
            1
        );
        let peer = &server.peers[0];
        let slot = peer.slot;
        let sequence = peer.sequence;
        let token = peer.token;
        let client = clients.iter().find(|c| c.slot == Some(slot)).unwrap();
        client
            .socket
            .send(&input_packet(
                token,
                sequence.saturating_sub(1),
                Input {
                    fire: true,
                    ..Default::default()
                },
            ))
            .unwrap();
        std::thread::sleep(Duration::from_millis(1));
        server.update(Input::default()).unwrap();
        assert!(
            !server.peers[0].input.fire,
            "stale inputs must not replay shots"
        );
        server.peers[0].last_seen = Instant::now() - TIMEOUT - Duration::from_secs(1);
        server.update(Input::default()).unwrap();
        assert!(!server.world.actors[slot].human);
        assert!(!server.world.actors[slot].active);
        assert_eq!(server.peer_count(), PLAYERS - 2);
    }
    #[test]
    fn prediction_ack_roundtrip_stays_within_mtu_and_rejects_invalid_state() {
        let mut world = World {
            event: "가".repeat(100),
            ..Default::default()
        };
        world.actors[6].vy = 4.75;
        world.actors[6].combat.slow = 1.5;
        world.actors[6].combat.speed_up = 5.0;
        world.actors[6].combat.stun = 0.25;
        world.actors[6].combat.sleep = 0.5;
        world.actors[6].combat.dash = 0.55;
        let packet = peer_snapshot(
            &world,
            42,
            12,
            InputAck {
                slot: 6,
                sequence: 9,
            },
        );
        assert!(packet.len() < MAX_PACKET);
        let (seq, decoded, ack) = read_snapshot(&packet, 42).unwrap();
        assert_eq!((seq, ack.slot, ack.sequence), (12, 6, 9));
        assert_eq!(decoded.actors[6].vy, 4.75);
        for (actual, expected) in [
            (decoded.actors[6].combat.slow, 1.5),
            (decoded.actors[6].combat.speed_up, 5.0),
            (decoded.actors[6].combat.stun, 0.25),
            (decoded.actors[6].combat.sleep, 0.5),
            (decoded.actors[6].combat.dash, 0.55),
        ] {
            assert!((actual - expected).abs() < 0.001);
        }
        let trailer = packet.len() - 35;
        let mut invalid = packet.clone();
        invalid[trailer] = PLAYERS as u8;
        assert!(read_snapshot(&invalid, 42).is_none());
        invalid = packet.clone();
        invalid[trailer + 9..trailer + 13].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(read_snapshot(&invalid, 42).is_none());
        invalid = packet;
        invalid[..4].copy_from_slice(b"TSBE");
        assert!(read_snapshot(&invalid, 42).is_none());
    }

    #[test]
    fn overtime_snapshot_tracks_presence_and_round_end() {
        let mut world = World::room(Default::default(), true);
        world.join_player(TEAM_SIZE);
        world.start();
        world.phase = Phase::Playing;
        world.remaining = DT * 0.5;
        world.actors[0].pos = world.points()[0];
        world.actors[TEAM_SIZE].pos = world.points()[0];
        world.update_objective();

        let packet = snapshot(&world, 42, 1);
        assert!(packet.len() < 1200);
        let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
        assert_eq!(decoded.phase, Phase::Playing);
        assert_eq!(decoded.remaining, 0.0);
        assert!(decoded.overtime);
        assert!(decoded.contested);
        assert_eq!(decoded.result, None);

        world.actors[0].pos.x += RADIUS + 0.1;
        world.update_objective();
        let (_, decoded, _) = read_snapshot(&snapshot(&world, 42, 2), 42).unwrap();
        assert_eq!(decoded.phase, Phase::Intermission);
        assert!(!decoded.overtime);
        assert_eq!(decoded.result, Some(1));
    }

    #[test]
    fn snapshot_roundtrip_and_mtu() {
        let mut world = World::default();
        world.start();
        world.actors[2].hp = 37;
        world.actors[2].pos = V3::new(1.0, 2.0, 3.0);
        world.extra = true;
        world.event = "가".repeat(80);
        world.actors[11].barrier = 9.6;
        world.actors[11].ghost_phase = crate::movement::PHASE_LINGER;
        world.actors[11].ability_cooldown = 7.6;
        world.actors[11].combat.ultimate = 99.9;
        world.actors[11].combat.giant = 5.0;
        world.actors[11].combat.guard = 70.0;
        world.actors[11].dancing = true;
        world.actors[2].crouching = true;
        world.actors[2].wave_ticks = WAVE_TICKS;
        world.actors[11].combat.regen = 4.0;
        let packet = snapshot(&world, 42, 8);
        assert!(packet.len() < 1200);
        let (seq, decoded, _) = read_snapshot(&packet, 42).unwrap();
        assert_eq!(seq, 8);
        assert_eq!(decoded.actors[2].hp, 37);
        assert!(decoded.actors[2].crouching);
        assert_eq!(decoded.actors[2].wave_ticks, WAVE_TICKS);
        assert!(!decoded.actors[11].crouching);
        assert_eq!(decoded.actors[2].pos, world.actors[2].pos);
        assert!(decoded.extra);
        assert_eq!(decoded.actors[11].character, world.actors[11].character);
        assert_eq!(decoded.actors[11].barrier, 9.6);
        assert_eq!(
            decoded.actors[11].ghost_phase,
            crate::movement::PHASE_LINGER
        );
        assert_eq!(decoded.actors[11].ability_cooldown, 7.6);
        assert_eq!(decoded.actors[11].combat.ultimate, 99.0);
        assert!(decoded.actors[11].combat.giant > 0.0);
        assert_eq!(decoded.actors[11].combat.guard, 70.0);
        assert!(decoded.actors[11].dancing);
        assert!(decoded.actors[11].combat.regen > 0.0);
        assert_eq!(decoded.actors[2].combat.regen, 0.0);
        world.actors[11].combat.regen = 0.0;
        let (_, expired, _) = read_snapshot(&snapshot(&world, 42, 9), 42).unwrap();
        assert_eq!(expired.actors[11].combat.regen, 0.0);
        assert!(read_snapshot(&packet, 43).is_none());
        for n in 0..packet.len() {
            assert!(read_snapshot(&packet[..n], 42).is_none());
        }
    }

    #[test]
    fn vivi_combo_snapshot_roundtrips_without_consuming_ammo_or_mtu() {
        let mut world = World::default();
        world.actors[0].character = 0;
        world.actors[0].ammo = 0;
        let size = snapshot(&world, 42, 1).len();
        for stage in 0..=2 {
            world.actors[0].combat.vivi_combo = stage;
            let packet = snapshot(&world, 42, 1);
            assert_eq!(packet.len(), size);
            let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
            assert_eq!(decoded.actors[0].combat.vivi_combo, stage);
            assert_eq!(decoded.actors[0].ammo, 0);
            assert_eq!(decoded.actors[0].combat.magic_ammo, 0);
        }
    }
    #[test]
    fn aya_snapshot_selects_enhanced_attack_after_skipped_packets() {
        use crate::motion::{Motion, MotionClock};
        let mut world = World::default();
        world.actors[0].character = 4;
        world.actors[0].ammo = 0;
        let mut clock = MotionClock::default();
        clock.observe(&world.actors[0], 0.0, 0, true);
        let size = snapshot(&world, 42, 1).len();
        for (shot, stage) in [(1, 1), (3, 3), (5, 2), (6, 3)] {
            world.actors[0].shot = shot;
            world.actors[0].combat.aya_combo = stage;
            let packet = snapshot(&world, 42, 1);
            assert_eq!(packet.len(), size);
            let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
            let a = &decoded.actors[0];
            assert_eq!(a.combat.aya_combo, stage);
            assert_eq!(a.combat.vivi_combo, 0);
            assert_eq!(a.combat.magic_ammo, 0);
            assert_eq!(
                clock.observe(a, 0.1, shot as u64, true).motion,
                if stage == 3 {
                    Motion::Attack2
                } else {
                    Motion::Attack
                }
            );
        }
        let mut old = snapshot(&world, 42, 1);
        old[..4].copy_from_slice(b"TSBI");
        assert!(read_snapshot(&old, 42).is_none());
    }
    #[test]
    fn naia_skill_snapshot_keeps_phase_clock_and_mtu_for_observers() {
        use crate::naia_skill::{Phase as SkillPhase, State, ACTIVE_SECONDS, START_SECONDS};
        let mut world = World::default();
        world.actors[0].character = 1;
        world.actors[0].ammo = 24;
        let size = snapshot(&world, 42, 1).len();
        for (phase, elapsed) in [
            (SkillPhase::Start, 0.4),
            (SkillPhase::Move, START_SECONDS + 0.2),
            (SkillPhase::Fire, START_SECONDS + 0.3),
            (SkillPhase::End, START_SECONDS + ACTIVE_SECONDS + 0.2),
            (SkillPhase::Ultimate, 2.6),
        ] {
            world.actors[0].combat.naia = State {
                phase,
                elapsed,
                pulse: 0.0,
            };
            let packet = snapshot(&world, 42, 1);
            assert_eq!(packet.len(), size);
            assert!(size <= MAX_PACKET);
            let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
            let a = &decoded.actors[0];
            assert_eq!(a.combat.naia.phase, phase);
            assert_eq!(a.combat.naia.elapsed, elapsed);
            assert_eq!(a.ammo, 24);
            assert_eq!(a.combat.vivi_combo, 0);
        }
        world.actors[0].combat.naia.elapsed = f32::NAN;
        assert!(read_snapshot(&snapshot(&world, 42, 1), 42).is_none());
        world.actors[0].combat.naia.elapsed = 1.0;
        world.actors[0].character = 0;
        world.actors[0].ammo = 0;
        // Vivi now owns this phase/clock slot; stale Naia state is not serialized.
        let (_, decoded, _) = read_snapshot(&snapshot(&world, 42, 1), 42).unwrap();
        assert_eq!(decoded.actors[0].combat.naia.phase, SkillPhase::Off);
        assert!(!decoded.actors[0].combat.vivi.active());
    }

    #[test]
    fn naia_wave_heading_survives_snapshot_without_growing_max_packet() {
        let mut world = World::default();
        for i in 0..MAX_PROJECTILES {
            let mut p = Projectile::visual(
                i as u32,
                i % PLAYERS,
                ProjectileKind::Wave,
                V3::new(2.0, 0.0, 3.0),
            );
            p.velocity = direction(i as f32 * 0.173, 0.0) * crate::naia_skill::WAVE_SPEED;
            world.projectiles.push(p);
        }
        let bytes = projectile_snapshot(&world, 42, 9);
        assert!(bytes.len() < MAX_PACKET);
        let (_, decoded) = read_projectiles(&bytes, 42).unwrap();
        for (a, b) in world.projectiles.iter().zip(&decoded) {
            assert_eq!(a.owner, b.owner);
            assert_eq!(a.kind, b.kind);
            assert!((a.velocity.normalized() - b.velocity.normalized()).length() < 0.026);
        }
    }
    #[test]
    fn empty_room_and_configured_bots_roundtrip_and_reject_old_protocol() {
        let mut world = World::room(crate::level::MapId::ElfCity, true);
        world.set_bot(6, Some(14));
        let mut packet = snapshot(&world, 42, 1);
        let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
        assert_eq!(decoded.team_count(0), 1);
        assert_eq!(decoded.team_count(1), 1);
        assert!(decoded.actors[0].human);
        assert!(decoded.actors[6].active && !decoded.actors[6].human);
        assert_eq!(decoded.actors[6].character, 14);
        assert!(decoded.actors[1..6].iter().all(|a| !a.active && !a.alive()));
        packet[..4].copy_from_slice(b"TSBD");
        assert!(read_snapshot(&packet, 42).is_none());
    }
    #[test]
    fn phase_skill_snapshot_roundtrips_timers_held_key_safe_exit_and_mtu() {
        let mut world = World::default();
        world.actors[0].character = 14;
        world.actors[0].ammo = kit(14).magazine;
        world.actors[0].phase_skill = crate::phase_skill::State {
            remaining: 1.5,
            cooldown: 0.0,
            down: true,
            safe_position: V3::new(1.0, 0.0, 3.0),
        };
        world.actors[11].character = 9;
        world.actors[11].ammo = kit(9).magazine;
        world.actors[11].phase_skill.cooldown = 12.0;
        let packet = snapshot(&world, 42, 1);
        assert!(packet.len() < MAX_PACKET);
        let (_, decoded, _) = read_snapshot(&packet, 42).unwrap();
        let phase = decoded.actors[0].phase_skill;
        assert!((phase.remaining - 1.5).abs() < DT);
        assert!(phase.down);
        assert_eq!(
            phase.safe_position,
            world.actors[0].phase_skill.safe_position
        );
        assert!((decoded.actors[11].phase_skill.cooldown - 12.0).abs() < DT);
        world.actors[0].phase_skill.cooldown = 1.0;
        assert!(read_snapshot(&snapshot(&world, 42, 1), 42).is_none());
        world.actors[0].phase_skill.cooldown = 0.0;
        world.actors[0].character = 3;
        assert!(read_snapshot(&snapshot(&world, 42, 1), 42).is_none());
        let mut old = packet;
        old[..4].copy_from_slice(b"TSBJ");
        assert!(read_snapshot(&old, 42).is_none());
    }

    #[test]
    fn udp_ghost_crossing_and_wall_contacts_reach_observer() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        let mut clients = vec![
            Client::connect(server.addr().unwrap()).unwrap(),
            Client::connect(server.addr().unwrap()).unwrap(),
        ];
        pump(&mut server, &mut clients, 12);
        let slot = clients[0].slot.unwrap();
        server.world.phase = Phase::Playing;
        for a in server.world.actors.iter_mut().filter(|a| a.active) {
            a.human = true;
        }
        server.world.actors[slot].character = 14;
        server.world.actors[slot].pos = V3::new(-8.4, 0.0, 8.55);
        let mut saw_contact = false;
        for _ in 0..28 {
            clients[0]
                .update(Input {
                    phase_shift: true,
                    forward: 1.0,
                    yaw: -std::f32::consts::FRAC_PI_2,
                    ..Default::default()
                })
                .unwrap();
            server.update(Input::default()).unwrap();
            clients[1].update(Input::default()).unwrap();
            let a = &clients[1].world.actors[slot];
            saw_contact |= a.ghost_phase > 0.0
                && !crate::ghost::wall_contacts(&clients[1].world.map, a.pos).is_empty();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            saw_contact,
            "observer must render authoritative wall contact"
        );
        assert!(clients[1].world.actors[slot].pos.x < -9.7);
        pump(&mut server, &mut clients, 25);
        assert_eq!(clients[1].world.actors[slot].ghost_phase, 0.0);
        assert!(clients[1].world.actors[slot].phase_skill.active());
        // Keep both peers connected through real expiry and the cooldown snapshot.
        pump(&mut server, &mut clients, 140);
        for client in &clients {
            let phase = client.world.actors[slot].phase_skill;
            assert!(!phase.active());
            assert!(phase.cooldown > 11.0 && phase.cooldown <= crate::phase_skill::COOLDOWN);
        }
    }
    #[test]
    fn input_rejects_nonfinite_and_trailing_data() {
        let mut p = input_packet(
            1,
            2,
            Input {
                yaw: f32::NAN,
                ..Default::default()
            },
        );
        assert!(read_input(&p).is_none());
        p = input_packet(
            1,
            2,
            Input {
                forward: 99.0,
                ..Default::default()
            },
        );
        assert_eq!(read_input(&p).unwrap().2.forward, 1.0);
        p.push(9);
        assert!(read_input(&p).is_none());
    }
    #[test]
    fn dance_and_crouch_input_roundtrip() {
        let mut p = input_packet(
            1,
            2,
            Input {
                dance: true,
                ..Default::default()
            },
        );
        assert!(read_input(&p).unwrap().2.dance);
        let flags = p.len() - 6; // Alternate fire followed by the viewport ratio.
        p[flags] |= 128;
        let input = read_input(&p).unwrap().2;
        assert!(input.dance && input.crouch);
        let p = input_packet(
            1,
            2,
            Input {
                crouch: true,
                ..Default::default()
            },
        );
        assert!(read_input(&p).unwrap().2.crouch);
    }
    #[test]
    fn udp_dance_reaches_observers_and_late_join_then_movement_cancels() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        let mut clients = vec![
            Client::connect(server.addr().unwrap()).unwrap(),
            Client::connect(server.addr().unwrap()).unwrap(),
        ];
        pump(&mut server, &mut clients, 12);
        let slot = clients[0].slot.unwrap();
        for _ in 0..10 {
            clients[0]
                .update(Input {
                    dance: true,
                    ..Default::default()
                })
                .unwrap();
            clients[1].update(Input::default()).unwrap();
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(server.world.actors[slot].dancing);
        assert!(clients[1].world.actors[slot].dancing);
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 12);
        assert!(clients[2].world.actors[slot].dancing);
        for _ in 0..10 {
            clients[0]
                .update(Input {
                    forward: 1.0,
                    ..Default::default()
                })
                .unwrap();
            clients[1].update(Input::default()).unwrap();
            server.update(Input::default()).unwrap();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!server.world.actors[slot].dancing);
        assert!(!clients[1].world.actors[slot].dancing);
    }
    #[test]
    fn udp_crouch_reaches_observers_and_late_join_then_releases() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        let mut clients = vec![Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        let slot = clients[0].slot.unwrap();
        server.world.phase = Phase::Playing;
        server.world.map.clear();
        for joined in [false, true] {
            if joined {
                clients.push(Client::connect(server.addr().unwrap()).unwrap());
            }
            for _ in 0..15 {
                clients[0]
                    .update(Input {
                        crouch: true,
                        ..Default::default()
                    })
                    .unwrap();
                for observer in clients.iter_mut().skip(1) {
                    observer.update(Input::default()).unwrap();
                }
                server.update(Input::default()).unwrap();
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(server.world.actors[slot].crouching);
            assert!(clients.iter().all(|c| c.world.actors[slot].crouching));
        }
        pump(&mut server, &mut clients, 15);
        assert!(!server.world.actors[slot].crouching);
        assert!(clients.iter().all(|c| !c.world.actors[slot].crouching));
    }
    #[test]
    fn projectile_packet_is_bounded_validated_and_preserves_signed_coordinates() {
        let w = World {
            projectiles: (0..MAX_PROJECTILES)
                .map(|i| {
                    Projectile::visual(
                        i as u32,
                        i % PLAYERS,
                        ProjectileKind::Boulder,
                        V3::new(-12.345, 2.0, 34.56),
                    )
                })
                .collect(),
            ..Default::default()
        };
        let packet = projectile_snapshot(&w, 44, 99);
        assert!(packet.len() < MAX_PACKET);
        let (seq, decoded) = read_projectiles(&packet, 44).unwrap();
        assert_eq!(seq, 99);
        assert_eq!(decoded.len(), MAX_PROJECTILES);
        assert!((decoded[0].pos.x + 12.35).abs() < 0.001);
        assert_eq!(decoded[0].kind, ProjectileKind::Boulder);
        assert!(read_projectiles(&packet, 45).is_none());
        for n in 0..packet.len() {
            assert!(read_projectiles(&packet[..n], 44).is_none());
        }
        let mut bad = packet.clone();
        bad[21] = 255;
        assert!(read_projectiles(&bad, 44).is_none());
        let mut bad = packet.clone();
        bad[27] = 255;
        assert!(read_projectiles(&bad, 44).is_none());
        let mut bad = packet;
        bad.extend([0]);
        assert!(read_projectiles(&bad, 44).is_none());
        let packet = input_packet(
            1,
            2,
            Input {
                ultimate: true,
                ..Default::default()
            },
        );
        assert!(read_input(&packet).unwrap().2.ultimate);
    }

    #[test]
    fn impact_packet_roundtrips_at_capacity_and_rejects_invalid_contacts() {
        let mut world = World {
            tick: 100,
            ..Default::default()
        };
        for i in 0..MAX_IMPACTS {
            world.record_impact(
                crate::impact::SurfaceHit {
                    distance: 1.0,
                    position: V3::new(-12.25, 1.5, i as f32),
                    face: (i % 6) as u8,
                    color: [130, 150, 170],
                },
                0.13,
            );
            if i % 4 != 0 {
                let hit = world.impacts.last_mut().unwrap();
                hit.kind = match i % 4 {
                    1 => ImpactKind::Slash,
                    2 => ImpactKind::PoisonDart,
                    _ => ImpactKind::Syringe,
                };
                hit.radius = if hit.kind == ImpactKind::Slash {
                    0.48
                } else {
                    0.13
                };
                hit.rotation = (i * 17) as u8;
            }
        }
        let packet = impact_snapshot(&world, 44, 99);
        assert!(packet.len() < MAX_PACKET);
        let decoded = read_impacts(&packet, 44).unwrap();
        assert_eq!(decoded.impacts, world.impacts);
        assert_eq!(decoded.sequence, 99);
        assert_eq!(decoded.round, world.round);
        assert_eq!(decoded.map, world.map_id);
        assert!(read_impacts(&packet, 45).is_none());
        let mut bad_kind = packet.clone();
        bad_kind[33 + 32] = 255; // Header + first contact's kind byte.
        assert!(read_impacts(&bad_kind, 44).is_none());
        for n in 0..packet.len() {
            assert!(read_impacts(&packet[..n], 44).is_none());
        }
        for invalid in 0..5 {
            let mut bad = world.clone();
            match invalid {
                0 => bad.impacts[0].face = 6,
                1 => bad.impacts[0].position.x = f32::NAN,
                2 => bad.impacts[0].radius = 1.0,
                3 => bad.impacts[0].tick = 101,
                _ => bad.impacts[0].id = bad.impacts[1].id,
            }
            assert!(read_impacts(&impact_snapshot(&bad, 44, 99), 44).is_none());
        }
        let mut extra = packet;
        extra.push(0);
        assert!(read_impacts(&extra, 44).is_none());
    }

    #[test]
    fn udp_impacts_reach_observers_and_late_joiners_then_expire() {
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients = vec![Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 12);
        server.world.record_impact(
            crate::impact::SurfaceHit {
                distance: 1.0,
                position: V3::new(-5.8, 1.5, -23.5),
                face: 1,
                color: [231, 235, 220],
            },
            0.13,
        );
        server.world.record_slash(
            crate::impact::SurfaceHit {
                distance: 1.0,
                position: V3::new(-5.8, 1.5, -22.0),
                face: 1,
                color: [231, 235, 220],
            },
            0.64,
            23,
        );
        for kind in [
            ProjectileKind::Needle,
            ProjectileKind::Poison,
            ProjectileKind::Syringe,
        ] {
            server.world.record_projectile_impact(
                crate::impact::SurfaceHit {
                    distance: 1.0,
                    position: V3::new(-5.8, 1.5, -21.0),
                    face: 1,
                    color: [231, 235, 220],
                },
                0.11,
                kind,
                V3::new(-1.0, 0.2, 0.3).normalized(),
            );
        }
        let expected = server.world.impacts.clone();
        pump(&mut server, &mut clients, 12);
        assert_eq!(clients[0].world.impacts, expected);
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, 12);
        assert!(clients.iter().all(|c| c.world.impacts == expected));
        // A repeated or out-of-order packet cannot overwrite newer contacts.
        let peer = &server.peers[0];
        server
            .socket
            .send_to(
                &impact_snapshot(&World::default(), peer.token, 1),
                peer.addr,
            )
            .unwrap();
        clients[0].update(Input::default()).unwrap();
        assert_eq!(clients[0].world.impacts, expected);
        pump(&mut server, &mut clients, 245);
        assert!(server.world.impacts.is_empty());
        assert!(clients.iter().all(|c| c.world.impacts.is_empty()));
    }
    #[test]
    fn udp_q_uses_server_charge_and_replicates_projectiles_and_damage() {
        let mut server = Server::bind("127.0.0.1:0", false).unwrap();
        // Combat fixture needs a populated enemy team to receive projectile hits.
        server.world = World::default();
        let mut clients = [Client::connect(server.addr().unwrap()).unwrap()];
        pump(&mut server, &mut clients, 15);
        clients[0].select_character(11);
        pump(&mut server, &mut clients, 10);
        server.start();
        server.world.phase = Phase::Playing;
        server.world.map.clear();
        for (i, a) in server.world.actors.iter_mut().enumerate() {
            a.human = true;
            a.shield = 0.0;
            a.pos = V3::new(50.0 + i as f32, 0.0, 40.0);
        }
        server.world.actors[0].pos = V3::default();
        server.world.actors[6].pos = V3::new(0.0, 0.0, 8.0);
        // An uncharged Q input cannot spawn an ultimate.
        clients[0]
            .update(Input {
                ultimate: true,
                ..Default::default()
            })
            .unwrap();
        server.update(Input::default()).unwrap();
        assert!(server.world.projectiles.is_empty());
        pump(&mut server, &mut clients, 5);
        server.world.actors[0].combat.ultimate = 100.0;
        let mut saw_projectile = false;
        // Butter now releases on Ultimate1_1's 2.5s event, not on Q press.
        for _ in 0..210 {
            clients[0]
                .update(Input {
                    ultimate: true,
                    ..Default::default()
                })
                .unwrap();
            server.update(Input::default()).unwrap();
            saw_projectile |= clients[0]
                .world
                .projectiles
                .iter()
                .any(|p| p.kind == ProjectileKind::Boulder);
            std::thread::sleep(Duration::from_millis(1));
        }
        pump(&mut server, &mut clients, 8);
        assert!(saw_projectile);
        assert!(server.world.actors[6].hp < MAX_HP);
        assert_eq!(clients[0].world.actors[6].hp, server.world.actors[6].hp);
        assert!(clients[0].world.actors[0].combat.ultimate < 5.0);
        assert_eq!(server.world.actors[0].shot, 1, "held Q cannot recast");
    }
}

#[cfg(test)]
mod erpin_wire_tests {
    use super::*;
    #[test]
    fn cake_action_and_pending_bolt_share_mtu_without_losing_release_latch() {
        let mut w = World::default();
        for a in &mut w.actors {
            a.character = 2;
            a.ammo = 20;
        }
        for state in [
            crate::erpin::State {
                eating: true,
                elapsed: 0.65,
                alt_down: true,
                shot_delay: 0.0,
            },
            crate::erpin::State {
                shot_delay: crate::erpin::shot_delay(),
                ..Default::default()
            },
        ] {
            for a in &mut w.actors {
                a.combat.erpin = state;
            }
            let packet = snapshot(&w, 44, 99);
            assert!(packet.len() < MAX_PACKET);
            let (_, got, _) = read_snapshot(&packet, 44).unwrap();
            for a in &got.actors {
                assert_eq!(a.combat.erpin.eating, state.eating);
                assert_eq!(a.combat.erpin.elapsed, state.elapsed);
                assert_eq!(a.combat.erpin.alt_down, state.alt_down);
                assert_eq!(a.combat.erpin.shot_delay, state.shot_delay);
            }
        }
    }
    #[test]
    fn remote_erpin_eating_cancel_and_debris_reach_owner_observer_and_late_joiner() {
        fn pump(server: &mut Server, clients: &mut [Client], input: Input, count: usize) {
            for _ in 0..count {
                for (i, c) in clients.iter_mut().enumerate() {
                    c.update(if i == 0 { input } else { Input::default() })
                        .unwrap();
                }
                server.update(Input::default()).unwrap();
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let mut server = Server::bind("127.0.0.1:0", true).unwrap();
        let mut clients = vec![
            Client::connect(server.addr().unwrap()).unwrap(),
            Client::connect(server.addr().unwrap()).unwrap(),
        ];
        pump(&mut server, &mut clients, Input::default(), 12);
        let owner = clients[0].slot.unwrap();
        server.world.phase = Phase::Playing;
        server.world.map.clear();
        for (i, a) in server.world.actors.iter_mut().enumerate() {
            a.active = i == owner || i == clients[1].slot.unwrap();
            a.human = a.active;
            if !a.active {
                a.hp = 0;
            }
        }
        let a = &mut server.world.actors[owner];
        a.character = 2;
        a.ammo = 20;
        a.hp = 40;
        a.shield = 0.0;
        a.pos = V3::default();
        a.cooldown = 0.0;
        pump(
            &mut server,
            &mut clients,
            Input {
                alt_fire: true,
                ..Default::default()
            },
            36,
        );
        assert!(server.world.actors[owner].combat.erpin.eating);
        for peer in &server.peers {
            assert!(
                read_snapshot(
                    &peer_snapshot(
                        &server.world,
                        peer.token,
                        server.sequence,
                        InputAck {
                            slot: peer.slot,
                            sequence: peer.sequence
                        }
                    ),
                    peer.token
                )
                .is_some(),
                "invalid authority snapshot for {}",
                peer.slot
            );
            assert!(
                read_portals(
                    &portal_snapshot(&server.world, peer.token, server.sequence),
                    peer.token
                )
                .is_some(),
                "invalid portal snapshot"
            );
        }
        for c in &clients {
            assert!(c.world.actors[owner].combat.erpin.eating);
        }
        assert_eq!(server.world.actors[owner].hp, 58);
        pump(
            &mut server,
            &mut clients,
            Input {
                fire: true,
                ..Default::default()
            },
            1,
        );
        pump(&mut server, &mut clients, Input::default(), 12);
        clients.push(Client::connect(server.addr().unwrap()).unwrap());
        pump(&mut server, &mut clients, Input::default(), 12);
        for c in &clients {
            assert!(!c.world.actors[owner].combat.erpin.eating);
            assert!(c
                .world
                .projectiles
                .iter()
                .any(|p| p.kind == ProjectileKind::Cake2));
            assert_eq!(c.world.actors[owner].hp, 58);
        }
    }

    #[test]
    fn dropped_cake_portions_are_replicated_in_the_existing_projectile_packet() {
        let mut w = World::default();
        for remaining in 1..=4 {
            w.projectiles.push(Projectile::visual(
                remaining as u32,
                0,
                ProjectileKind::cake(remaining),
                V3::new(0.2, 0.4, 0.6),
            ));
        }
        let (_, got) = read_projectiles(&projectile_snapshot(&w, 44, 99), 44).unwrap();
        for (i, p) in got.iter().enumerate() {
            assert_eq!(p.kind.cake_remaining(), Some(i as u8 + 1));
        }
    }
}

#[cfg(test)]
mod renua_tests;
#[cfg(test)]
mod hilde_tests;
#[cfg(test)]
mod shion_tests;

#[cfg(test)]
#[path = "net/komi_tests.rs"]
mod komi_tests;
#[cfg(test)]
#[path = "net/butter_tests.rs"]
mod butter_tests;

#[cfg(test)]
mod mayo_tests;

#[cfg(test)]
mod bella_tests;

#[cfg(test)]
mod picora_tests;

#[cfg(test)]
#[path = "net/erpin_skill_tests.rs"]
mod erpin_skill_tests;

#[cfg(test)]
mod chloe_tests;
