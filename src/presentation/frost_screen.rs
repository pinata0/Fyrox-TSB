//! Static crystalline glaze: no animated noise while the player's view is locked.
use super::*;
use fyrox::{
    gui::{image::ImageBuilder, widget::WidgetMaterial},
    material::shader::{Shader, ShaderResource},
};
#[derive(Debug, Default)]
pub(super) struct FrostScreen {
    pub root: Handle<UiNode>,
}
impl FrostScreen {
    pub fn new(ui: &mut UserInterface) -> Self {
        let material = MaterialResource::new(Material::from_shader(ShaderResource::new_embedded(
            Shader::from_string(include_str!("frost_screen.shader")).expect("Frost screen shader"),
        )));
        let root = ImageBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_hit_test_visibility(false)
                .with_material(WidgetMaterial(material)),
        )
        .with_sync_with_texture_size(false)
        .build(&mut ui.build_ctx())
        .to_base();
        Self { root }
    }
    pub fn render(&self, ui: &UserInterface, actor: &Actor, show: bool) {
        let active = show && actor.alive() && actor.combat.frozen > 0.0;
        visible(ui, self.root, active);
        if active {
            let size = ui.screen_size();
            place(ui, self.root, 0.0, 0.0, size.x, size.y);
        }
    }
}
