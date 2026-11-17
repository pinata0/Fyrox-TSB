//! Quick communication: clockwise wheel geometry and server-authoritative actions.
use crate::game::{Actor, Phase, World, DT};

pub const WAVE_TICKS: u8 = 108;
pub const QUICK_COOLDOWN: f32 = 0.75;
pub const DEAD_ZONE: f32 = 58.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Communication {
    Emote,
    Hello,
    NeedHealing,
    GroupUp,
    UltimateStatus,
    VoiceLine,
    Acknowledge,
    Thanks,
}
impl Communication {
    pub const ALL: [Self; 8] = [
        Self::Emote,
        Self::Hello,
        Self::NeedHealing,
        Self::GroupUp,
        Self::UltimateStatus,
        Self::VoiceLine,
        Self::Acknowledge,
        Self::Thanks,
    ];
    pub fn from_wire(value: u8) -> Option<Self> {
        Self::ALL.get(value as usize).copied()
    }
    pub fn label(self, korean: bool) -> &'static str {
        let labels = if korean {
            [
                "감정 표현",
                "안녕",
                "치유 요청",
                "집결",
                "궁극기 상태",
                "음성 대사",
                "응답",
                "감사",
            ]
        } else {
            [
                "EMOTE",
                "HELLO",
                "NEED HEALING",
                "GROUP UP",
                "ULTIMATE STATUS",
                "VOICE LINE",
                "ACKNOWLEDGE",
                "THANKS",
            ]
        };
        labels[self as usize]
    }
    pub fn message(self, actor: &Actor) -> String {
        match self {
            Self::Emote => "감정 표현: 춤을 춥니다.".into(),
            Self::Hello => "안녕하세요!".into(),
            Self::NeedHealing => "치유가 필요합니다!".into(),
            Self::GroupUp => "제게 집결해 주세요!".into(),
            Self::UltimateStatus if actor.combat.ultimate >= 100.0 => {
                "궁극기가 준비되었습니다! (100%)".into()
            }
            Self::UltimateStatus => format!(
                "궁극기 충전 중입니다. ({:.0}%)",
                actor.combat.ultimate.clamp(0.0, 100.0).floor()
            ),
            Self::VoiceLine => "음성 대사 준비 중".into(),
            Self::Acknowledge => "알겠습니다!".into(),
            Self::Thanks => "감사합니다!".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommunicationResult {
    Accepted,
    Unavailable,
    Cooldown,
    VoiceUnavailable,
}
impl CommunicationResult {
    pub fn from_wire(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Accepted),
            1 => Some(Self::Unavailable),
            2 => Some(Self::Cooldown),
            3 => Some(Self::VoiceUnavailable),
            _ => None,
        }
    }
    pub fn message(self, korean: bool) -> &'static str {
        match (self, korean) {
            (Self::Accepted, _) => "",
            (Self::Unavailable, true) => "지금은 감정 표현을 할 수 없습니다.",
            (Self::Unavailable, false) => "Emote unavailable right now.",
            (Self::Cooldown, true) => "잠시 후 다시 사용해 주세요.",
            (Self::Cooldown, false) => "Please wait a moment.",
            (Self::VoiceUnavailable, true) => "음성 대사 준비 중",
            (Self::VoiceUnavailable, false) => "Voice lines coming soon",
        }
    }
}

impl World {
    pub fn communicate(
        &mut self,
        slot: usize,
        action: Communication,
    ) -> Result<String, CommunicationResult> {
        let Some(a) = self.actors.get_mut(slot).filter(|a| a.human) else {
            return Err(CommunicationResult::Unavailable);
        };
        if action == Communication::VoiceLine {
            return Err(CommunicationResult::VoiceUnavailable);
        }
        if a.communication_cooldown > 0.0 {
            return Err(CommunicationResult::Cooldown);
        }
        let can_animate = a.alive()
            && !a.combat.disabled()
            && !a.combat.naia.active()
            && a.combat.cast.is_none()
            && a.combat.dash <= 0.0
            && a.reload <= 0.0
            && a.hurt <= 0.0;
        if action == Communication::Emote
            && (!can_animate
                || a.crouching
                || !crate::movement::grounded(&self.map, a.pos)
                || matches!(self.phase, Phase::Intermission | Phase::Finished))
        {
            return Err(CommunicationResult::Unavailable);
        }
        match action {
            Communication::Emote => {
                a.dancing = true;
                a.wave_ticks = 0;
            }
            Communication::Hello if can_animate => {
                a.dancing = false;
                a.wave_ticks = WAVE_TICKS;
            }
            _ => {}
        }
        a.communication_cooldown = QUICK_COOLDOWN;
        Ok(action.message(a))
    }
}

#[derive(Debug, Default)]
pub struct CommunicationWheel {
    pub open: bool,
    pub x: f32,
    pub y: f32,
}
impl CommunicationWheel {
    pub fn begin(&mut self) {
        self.open = true;
        self.x = 0.0;
        self.y = 0.0;
    }
    pub fn drag(&mut self, dx: f32, dy: f32) {
        if !self.open || !dx.is_finite() || !dy.is_finite() {
            return;
        }
        self.x += dx;
        self.y += dy;
        let distance = self.x.hypot(self.y);
        if distance > 250.0 {
            self.x *= 250.0 / distance;
            self.y *= 250.0 / distance;
        }
    }
    pub fn selected(&self) -> Option<Communication> {
        if !self.open || self.x.hypot(self.y) < DEAD_ZONE {
            return None;
        }
        let angle = self.x.atan2(-self.y).rem_euclid(std::f32::consts::TAU);
        let index =
            ((angle + std::f32::consts::FRAC_PI_8) / std::f32::consts::FRAC_PI_4) as usize % 8;
        Some(Communication::ALL[index])
    }
    pub fn release(&mut self) -> Option<Communication> {
        let result = self.selected();
        self.cancel();
        result
    }
    pub fn cancel(&mut self) {
        self.open = false;
        self.x = 0.0;
        self.y = 0.0;
    }
}

/// Smooth lift/lower envelope and wrist sway shared by first- and third-person gestures.
pub fn wave_pose(ticks: u8) -> (f32, f32) {
    let elapsed = (WAVE_TICKS.saturating_sub(ticks)) as f32 * DT;
    let remaining = ticks as f32 * DT;
    let blend = (elapsed / 0.25).min(remaining / 0.3).clamp(0.0, 1.0);
    let blend = blend * blend * (3.0 - 2.0 * blend);
    (blend, (elapsed * 17.0).sin() * 0.32 * blend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Input, PLAYERS};
    #[test]
    fn wheel_clockwise_order_deadzone_cancel_and_one_shot_release() {
        let mut wheel = CommunicationWheel::default();
        wheel.begin();
        assert_eq!(wheel.release(), None);
        for (i, action) in Communication::ALL.iter().enumerate() {
            wheel.begin();
            let angle = i as f32 * std::f32::consts::FRAC_PI_4;
            wheel.drag(angle.sin() * 180.0, -angle.cos() * 180.0);
            assert_eq!(wheel.selected(), Some(*action));
            assert_eq!(wheel.release(), Some(*action));
            assert_eq!(wheel.release(), None);
        }
        wheel.begin();
        wheel.drag(100.0, 0.0);
        wheel.drag(-100.0, 0.0);
        assert_eq!(wheel.selected(), None);
        wheel.drag(f32::NAN, 0.0);
        assert_eq!(wheel.selected(), None);
        wheel.drag(100.0, 0.0);
        wheel.cancel();
        assert_eq!(wheel.release(), None);
    }
    #[test]
    fn authority_reports_real_charge_and_voice_placeholder_has_no_effect() {
        let mut w = World::default();
        w.actors[0].human = true;
        w.actors[0].combat.ultimate = 73.9;
        assert!(w
            .communicate(0, Communication::UltimateStatus)
            .unwrap()
            .contains("73%"));
        assert_eq!(w.actors[0].combat.ultimate, 73.9);
        assert_eq!(
            w.communicate(0, Communication::Hello),
            Err(CommunicationResult::Cooldown)
        );
        for _ in 0..46 {
            w.tick([Input::default(); PLAYERS]);
        }
        w.actors[0].combat.ultimate = 100.0;
        assert!(w
            .communicate(0, Communication::UltimateStatus)
            .unwrap()
            .contains("준비"));
        assert_eq!(
            w.communicate(0, Communication::VoiceLine),
            Err(CommunicationResult::VoiceUnavailable)
        );
        assert_eq!(w.actors[0].wave_ticks, 0);
        assert!(!w.actors[0].dancing);
        assert_eq!(
            w.communicate(1, Communication::Thanks),
            Err(CommunicationResult::Unavailable)
        );
    }
    #[test]
    fn hello_animates_expires_and_combat_cancels_without_blocking_calls_when_dead() {
        let mut w = World::default();
        w.actors[0].human = true;
        w.communicate(0, Communication::Hello).unwrap();
        assert_eq!(w.actors[0].wave_ticks, WAVE_TICKS);
        for _ in 0..WAVE_TICKS {
            w.tick([Input::default(); PLAYERS]);
        }
        assert_eq!(w.actors[0].wave_ticks, 0);
        w.communicate(0, Communication::Hello).unwrap();
        let mut inputs = [Input::default(); PLAYERS];
        inputs[0].fire = true;
        w.tick(inputs);
        assert_eq!(w.actors[0].wave_ticks, 0);
        w.actors[0].hp = 0;
        w.actors[0].communication_cooldown = 0.0;
        assert!(w.communicate(0, Communication::Hello).is_ok());
        assert_eq!(w.actors[0].wave_ticks, 0);
        w.actors[0].communication_cooldown = 0.0;
        assert_eq!(
            w.communicate(0, Communication::Emote),
            Err(CommunicationResult::Unavailable)
        );
    }
}
