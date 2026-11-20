//! Compact player card: existing 2D art, authoritative health and recovery status.
use super::*;
use fyrox::{
    asset::manager::ResourceManager,
    core::algebra::Matrix3,
    gui::{
        canvas::CanvasBuilder,
        image::{ImageBuilder, ImageMessage},
        vector_image::{Primitive, VectorImageBuilder},
    },
};

const WHITE: [u8; 3] = [245, 250, 255];
const CYAN: [u8; 3] = [111, 222, 255];
const MUTED: [u8; 3] = [159, 178, 195];
const SHIELD_YELLOW: [u8; 3] = [255, 220, 75];
const BAR_WIDTH: f32 = 258.0;
const PORTRAIT_WIDTH: f32 = 90.0;
const PORTRAIT_HEIGHT: f32 = 104.0;

#[derive(Debug, Default)]
pub(super) struct PlayerInfoUi {
    root: Handle<UiNode>,
    portrait: Handle<UiNode>,
    recovery_icon: Handle<UiNode>,
    recovery: Handle<UiNode>,
    health: Handle<UiNode>,
    maximum: Handle<UiNode>,
    fill: Handle<UiNode>,
    shield_fill: Handle<UiNode>,
    shield_health: Handle<UiNode>,
    name: Handle<UiNode>,
    textures: Vec<TextureResource>,
}

fn polygon(ctx: &mut BuildContext, points: &[(f32, f32)], color: Color) -> Handle<UiNode> {
    let points: Vec<_> = points.iter().map(|&(x, y)| Vector2::new(x, y)).collect();
    let primitives = (1..points.len() - 1)
        .map(|i| Primitive::Triangle {
            points: [points[0], points[i], points[i + 1]],
        })
        .collect();
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}

fn rectangle(ctx: &mut BuildContext, color: Color) -> Handle<UiNode> {
    BorderBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_background(Brush::Solid(color).into()),
    )
    .with_corner_radius(1.0.into())
    .build(ctx)
    .to_base()
}

impl PlayerInfoUi {
    pub fn new(ui: &mut UserInterface, manager: &ResourceManager) -> Self {
        let textures = portraits::load(manager);
        let ctx = &mut ui.build_ctx();
        // A flat top contains the close-up's opaque corners inside the frame.
        let frame = polygon(
            ctx,
            &[
                (0.0, 0.0),
                (100.0, 0.0),
                (100.0, 112.0),
                (50.0, 136.0),
                (0.0, 112.0),
            ],
            rgb(CYAN),
        );
        let inset = polygon(
            ctx,
            &[
                (3.0, 3.0),
                (97.0, 3.0),
                (97.0, 110.0),
                (50.0, 132.0),
                (3.0, 110.0),
            ],
            Color::from_rgba(22, 41, 61, 242),
        );
        let portrait = ImageBuilder::new(WidgetBuilder::new().with_hit_test_visibility(false))
            .with_texture(textures[0].clone())
            .with_sync_with_texture_size(false)
            .build(ctx)
            .to_base();
        let readout_bg = rectangle(ctx, Color::from_rgba(22, 35, 52, 160));
        let bar_frame = rectangle(ctx, Color::from_rgba(18, 30, 45, 235));
        let track = rectangle(ctx, Color::from_rgba(79, 99, 119, 220));
        let fill = rectangle(ctx, rgb(WHITE));
        let shield_fill = rectangle(ctx, rgb(SHIELD_YELLOW));
        let name_bg = rectangle(ctx, Color::from_rgba(22, 35, 52, 195));
        let underline = rectangle(ctx, rgb(CYAN));
        let recovery_bg = rectangle(ctx, Color::from_rgba(22, 35, 52, 200));
        // A medical cross drawn in geometry stays crisp at every HUD scale.
        let recovery_icon = VectorImageBuilder::new(
            WidgetBuilder::new()
                .with_hit_test_visibility(false)
                .with_foreground(Brush::Solid(rgb(MUTED)).into()),
        )
        .with_primitives(vec![
            Primitive::Line {
                begin: Vector2::new(10.0, 0.0),
                end: Vector2::new(10.0, 20.0),
                thickness: 6.0,
            },
            Primitive::Line {
                begin: Vector2::new(0.0, 10.0),
                end: Vector2::new(20.0, 10.0),
                thickness: 6.0,
            },
        ])
        .build(ctx)
        .to_base();
        let recovery = label(ctx, "", 13.0, MUTED);
        let health = label(ctx, "", 35.0, WHITE);
        let maximum = label(ctx, "", 19.0, MUTED);
        let shield_health = label(ctx, "", 23.0, SHIELD_YELLOW);
        let name = label(ctx, "", 17.0, WHITE);
        let children = vec![
            frame,
            inset,
            portrait,
            readout_bg,
            bar_frame,
            track,
            fill,
            shield_fill,
            name_bg,
            underline,
            recovery_bg,
            recovery_icon,
            recovery,
            health,
            maximum,
            shield_health,
            name,
        ];
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(384.0)
                .with_height(140.0)
                .with_hit_test_visibility(false)
                .with_children(children),
        )
        .build(ctx)
        .to_base();
        for (node, x, y, w, h) in [
            (frame, 0.0, 0.0, 100.0, 136.0),
            (inset, 0.0, 0.0, 100.0, 136.0),
            (portrait, 5.0, 8.0, PORTRAIT_WIDTH, PORTRAIT_HEIGHT),
            (recovery_bg, 112.0, 1.0, 268.0, 27.0),
            (recovery_icon, 118.0, 4.0, 21.0, 21.0),
            (recovery, 146.0, 3.0, 225.0, 24.0),
            (readout_bg, 112.0, 30.0, 268.0, 43.0),
            (health, 119.0, 25.0, 79.0, 48.0),
            (maximum, 202.0, 40.0, 70.0, 30.0),
            (shield_health, 280.0, 35.0, 95.0, 35.0),
            (bar_frame, 112.0, 75.0, 268.0, 27.0),
            (track, 117.0, 80.0, BAR_WIDTH, 17.0),
            (fill, 117.0, 80.0, BAR_WIDTH, 17.0),
            (shield_fill, 117.0, 80.0, 0.0, 17.0),
            (name_bg, 112.0, 106.0, 268.0, 29.0),
            (name, 120.0, 108.0, 252.0, 25.0),
            (underline, 112.0, 135.0, 268.0, 2.0),
        ] {
            place(ctx.inner(), node, x, y, w, h);
        }
        Self {
            root,
            portrait,
            recovery_icon,
            recovery,
            health,
            maximum,
            fill,
            shield_fill,
            shield_health,
            name,
            textures,
        }
    }

    pub fn labels(&self) -> [Handle<UiNode>; 5] {
        [
            self.recovery,
            self.health,
            self.maximum,
            self.shield_health,
            self.name,
        ]
    }

    pub fn render(&self, rt: &Runtime, ui: &UserInterface, show: bool) {
        visible(ui, self.root, show);
        if !show {
            return;
        }
        let size = ui.screen_size();
        let scale = (size.x / 1280.0).min(size.y / 800.0).clamp(0.65, 1.25);
        place(
            ui,
            self.root,
            26.0 * scale,
            size.y - 196.0 * scale,
            384.0,
            140.0,
        );
        ui.send(
            self.root,
            // Keep the canvas layout in design pixels. Scaling its layout makes
            // Fyrox constrain the child clip rectangle twice below 100% scale.
            WidgetMessage::RenderTransform(Matrix3::new_scaling(scale)),
        );
        let slot = rt.session.slot();
        let actor = &rt.session.world().actors[slot];
        let texture = &self.textures[actor.character as usize];
        ui.send(self.portrait, ImageMessage::Texture(Some(texture.clone())));
        let uv = portraits::uv(
            actor.character as usize,
            texture,
            PORTRAIT_WIDTH,
            PORTRAIT_HEIGHT,
        );
        ui.send(self.portrait, ImageMessage::UvRect(uv));
        let recovering = actor.alive() && actor.combat.regen > 0.0;
        let recovery_color = if recovering { MINT } else { MUTED };
        for node in [self.recovery_icon, self.recovery] {
            ui.send(
                node,
                WidgetMessage::Foreground(Brush::Solid(rgb(recovery_color)).into()),
            );
        }
        text(
            ui,
            self.recovery,
            match (rt.korean_font, recovering) {
                (true, true) => "자동회복 ON",
                (true, false) => "자동회복 OFF",
                (false, true) => "AUTO HEAL ON",
                (false, false) => "AUTO HEAL OFF",
            },
        );
        let maximum = actor.max_hp();
        let hp = actor.hp.min(maximum);
        let shield = if actor.alive() {
            actor.shield_health()
        } else {
            0.0
        };
        // Reserve capacity for the shield so full HP never hides the yellow segment.
        let capacity = maximum as f32 + shield;
        let hp_width = BAR_WIDTH * hp as f32 / capacity;
        text(ui, self.shield_health, format!("+{}", shield.ceil() as u16));
        visible(ui, self.shield_health, shield > 0.0);
        text(ui, self.health, hp.to_string());
        text(ui, self.maximum, format!("/ {maximum}"));
        let color = if hp <= maximum / 4 {
            [255, 113, 120]
        } else {
            WHITE
        };
        ui.send(
            self.health,
            WidgetMessage::Foreground(Brush::Solid(rgb(color)).into()),
        );
        ui.send(
            self.fill,
            WidgetMessage::Background(Brush::Solid(rgb(color)).into()),
        );
        place(ui, self.fill, 117.0, 80.0, hp_width, 17.0);
        visible(ui, self.fill, hp > 0);
        place(
            ui,
            self.shield_fill,
            117.0 + hp_width,
            80.0,
            BAR_WIDTH * shield / capacity,
            17.0,
        );
        visible(ui, self.shield_fill, shield > 0.0);
        ui.send(
            self.portrait,
            WidgetMessage::Opacity(Some(if actor.alive() { 1.0 } else { 0.4 })),
        );
        text(ui, self.name, rt.session.player_name(slot));
    }
}
