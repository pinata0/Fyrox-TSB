//! Local Shion aim zoom. It changes the camera and scope HUD, never authoritative combat.
use super::*;

pub(super) const SHION: u8 = 14;
pub(super) const NORMAL_FOV: f32 = 80.0;
const ZOOM_FOV: f32 = 35.0;

impl Runtime {
    pub(super) fn controls_active(&self) -> bool {
        !self.home_open
            && !self.settings_menu.open
            && !self.character_help.open
            && !self.lobby.open
            && self.grabbed
            && !self.chat.open
            && !self.selecting
            && !self.wheel.open
            && self.session.world().actors[self.session.slot()].alive()
    }

    pub(super) fn can_zoom(&self) -> bool {
        let world = self.session.world();
        let actor = &world.actors[self.session.slot()];
        self.controls_active()
            && self.error.is_none()
            && matches!(world.phase, Phase::Setup | Phase::Playing)
            && actor.character == SHION
            && !actor.combat.view_locked()
            && actor.combat.polymorph <= 0.0
            && !actor.dancing
            && actor.wave_ticks == 0
    }

    pub(super) fn set_zoom(&mut self, pressed: bool) {
        self.keys.zoom = pressed && self.can_zoom();
    }

    pub(super) fn zoomed(&self) -> bool {
        self.keys.zoom && self.can_zoom()
    }

    pub(super) fn gameplay_fov(&self) -> f32 {
        let base = if self.zoomed() { ZOOM_FOV } else { NORMAL_FOV };
        // At full slow, 80 -> 74 degrees; preserve the scope's relative magnification.
        (base * (1.0 - 0.075 * slow_effects::view_strength(self))).to_radians()
    }

    pub(super) fn look_sensitivity(&self) -> f32 {
        // Preserve mouse motion near the crosshair in screen space as magnification changes.
        trickcal_shooting_balddagu::settings::BASE_LOOK_SENSITIVITY
            * self.settings_menu.settings.mouse_sensitivity
            * (self.gameplay_fov() * 0.5).tan()
            / (NORMAL_FOV.to_radians() * 0.5).tan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> Runtime {
        let mut world = World::default();
        world.start();
        world.actors[0].character = SHION;
        let mut rt = Runtime::new(Session::Solo(Box::new(world)), None, true);
        rt.settings_menu.settings = Default::default();
        rt.grabbed = true;
        rt
    }

    fn world(rt: &mut Runtime) -> &mut World {
        let Session::Solo(world) = &mut rt.session else {
            unreachable!()
        };
        world
    }

    #[test]
    fn only_shion_zooms_while_right_button_is_held() {
        let mut rt = runtime();
        for id in 0..ROSTER.len() as u8 {
            world(&mut rt).actors[0].character = id;
            rt.set_zoom(true);
            assert_eq!(rt.zoomed(), id == SHION);
            if id == SHION {
                assert!((rt.gameplay_fov().to_degrees() - 35.0).abs() < 0.001);
                assert!(rt.look_sensitivity() < 0.0022);
                rt.keys.fire = true;
                assert!(rt.controls_active() && rt.zoomed());
            }
            rt.set_zoom(false);
            assert!(!rt.zoomed());
            assert!((rt.gameplay_fov().to_degrees() - 80.0).abs() < 0.001);
            assert!((rt.look_sensitivity() - 0.0022).abs() < 0.000001);
        }
    }

    #[test]
    fn menus_death_emotes_and_round_end_block_zoom() {
        let interruptions: [fn(&mut Runtime); 11] = [
            |rt| rt.settings_menu.open = true,
            |rt| rt.home_open = true,
            |rt| rt.grabbed = false,
            |rt| rt.chat.open = true,
            |rt| rt.selecting = true,
            |rt| rt.wheel.begin(),
            |rt| world(rt).actors[0].hp = 0,
            |rt| world(rt).actors[0].dancing = true,
            |rt| world(rt).actors[0].wave_ticks = 90,
            |rt| world(rt).phase = Phase::Intermission,
            |rt| rt.error = Some("Disconnected".into()),
        ];
        for interrupt in interruptions {
            let mut rt = runtime();
            rt.set_zoom(true);
            assert!(rt.zoomed());
            interrupt(&mut rt);
            assert!(!rt.zoomed());
            assert!((rt.gameplay_fov().to_degrees() - 80.0).abs() < 0.001);
            rt.set_zoom(true);
            assert!(!rt.keys.zoom);
        }
    }

    #[test]
    fn saved_sensitivity_scales_normal_and_zoomed_aim() {
        let mut rt = runtime();
        let normal = rt.look_sensitivity();
        rt.set_zoom(true);
        let zoom = rt.look_sensitivity();
        rt.settings_menu.settings.set_sensitivity(2.35);
        assert!((rt.look_sensitivity() - zoom * 2.35).abs() < 0.000001);
        rt.set_zoom(false);
        assert!((rt.look_sensitivity() - normal * 2.35).abs() < 0.000001);
        rt.settings_menu.open = true;
        assert!(!rt.controls_active());
    }

    #[test]
    fn slows_zoom_gently_and_restore_across_camera_and_menu_transitions() {
        let mut rt = runtime();
        world(&mut rt).actors[0].combat.slow = 2.0;
        assert!((rt.gameplay_fov().to_degrees() - 74.0).abs() < 0.001);
        rt.set_zoom(true);
        assert!((rt.gameplay_fov().to_degrees() - 32.375).abs() < 0.001);
        rt.set_zoom(false);
        world(&mut rt).actors[0].combat.slow = 0.0;
        world(&mut rt).actors[0].combat.rain_slow = 2.0;
        assert!(rt.gameplay_fov().to_degrees() > 74.0);
        assert!(rt.gameplay_fov().to_degrees() < 80.0);
        for interrupt in [
            (|rt: &mut Runtime| world(rt).actors[0].dancing = true) as fn(&mut Runtime),
            |rt| world(rt).actors[0].hp = 0,
            |rt| world(rt).actors[0].active = false,
            |rt| rt.selecting = true,
            |rt| rt.settings_menu.open = true,
            |rt| rt.character_help.open = true,
            |rt| rt.keys.score = true,
            |rt| rt.home_open = true,
            |rt| rt.lobby.open = true,
            |rt| rt.grabbed = false,
            |rt| world(rt).phase = Phase::Intermission,
        ] {
            let mut rt = runtime();
            world(&mut rt).actors[0].combat.slow = 2.0;
            interrupt(&mut rt);
            assert!((rt.gameplay_fov().to_degrees() - 80.0).abs() < 0.001);
        }
        world(&mut rt).actors[0].combat.rain_slow = 0.0;
        assert!((rt.gameplay_fov().to_degrees() - 80.0).abs() < 0.001);
    }
}
