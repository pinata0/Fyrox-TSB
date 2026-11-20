use super::*;
use fyrox::gui::{
    canvas::CanvasBuilder,
    vector_image::{Primitive, VectorImageBuilder},
    HorizontalAlignment,
};
use std::f32::consts::{FRAC_PI_4, FRAC_PI_8, TAU};
use trickcal_shooting_balddagu::communication::Communication;

#[derive(Debug, Default)]
pub(super) struct CommunicationUi {
    dim: Handle<UiNode>,
    root: Handle<UiNode>,
    title: Handle<UiNode>,
    sectors: [Handle<UiNode>; 8],
    icons: [Handle<UiNode>; 8],
    labels: [Handle<UiNode>; 8],
    center: Handle<UiNode>,
    detail: Handle<UiNode>,
    hint: Handle<UiNode>,
    pointer: Handle<UiNode>,
    toast: Handle<UiNode>,
}
fn vector(ctx: &mut BuildContext, primitives: Vec<Primitive>, color: Color) -> Handle<UiNode> {
    VectorImageBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(color).into()),
    )
    .with_primitives(primitives)
    .build(ctx)
    .to_base()
}
fn centered(ctx: &mut BuildContext, size: f32) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_hit_test_visibility(false)
            .with_foreground(Brush::Solid(rgb(CREAM)).into()),
    )
    .with_font_size(size.into())
    .with_horizontal_text_alignment(HorizontalAlignment::Center)
    .with_shadow(true)
    .build(ctx)
    .to_base()
}
fn point(angle: f32, radius: f32) -> Vector2<f32> {
    Vector2::new(300.0 + angle.sin() * radius, 300.0 - angle.cos() * radius)
}
fn sector(index: usize) -> Vec<Primitive> {
    let start = index as f32 * FRAC_PI_4 - FRAC_PI_8 + 0.014;
    let step = (FRAC_PI_4 - 0.028) / 16.0;
    let mut shapes = Vec::new();
    for i in 0..16 {
        let a = start + i as f32 * step;
        let b = a + step;
        let (a0, a1, b0, b1) = (
            point(a, 82.0),
            point(a, 236.0),
            point(b, 82.0),
            point(b, 236.0),
        );
        shapes.push(Primitive::Triangle {
            points: [a0, a1, b1],
        });
        shapes.push(Primitive::Triangle {
            points: [a0, b1, b0],
        });
    }
    shapes
}
fn line(a: (f32, f32), b: (f32, f32)) -> Primitive {
    Primitive::Line {
        begin: Vector2::new(a.0, a.1),
        end: Vector2::new(b.0, b.1),
        thickness: 2.7,
    }
}
fn circle(x: f32, y: f32, r: f32) -> Primitive {
    Primitive::WireCircle {
        center: Vector2::new(x, y),
        radius: r,
        thickness: 2.5,
        segments: 36,
    }
}
fn icon(action: Communication) -> Vec<Primitive> {
    match action {
        Communication::Emote => vec![
            circle(20.0, 20.0, 16.0),
            circle(14.0, 16.0, 1.0),
            circle(26.0, 16.0, 1.0),
            line((12.0, 24.0), (16.0, 28.0)),
            line((16.0, 28.0), (24.0, 28.0)),
            line((24.0, 28.0), (28.0, 24.0)),
        ],
        Communication::Hello => {
            let points = [
                (9.0, 33.0),
                (4.0, 23.0),
                (7.0, 20.0),
                (12.0, 25.0),
                (10.0, 11.0),
                (13.0, 10.0),
                (16.0, 22.0),
                (16.0, 5.0),
                (20.0, 5.0),
                (21.0, 21.0),
                (24.0, 7.0),
                (28.0, 8.0),
                (26.0, 23.0),
                (31.0, 14.0),
                (34.0, 16.0),
                (30.0, 31.0),
                (25.0, 36.0),
                (15.0, 37.0),
                (9.0, 33.0),
            ];
            points.windows(2).map(|p| line(p[0], p[1])).collect()
        }
        Communication::NeedHealing => vec![
            line((20.0, 6.0), (20.0, 34.0)),
            line((6.0, 20.0), (34.0, 20.0)),
            circle(20.0, 20.0, 18.0),
        ],
        Communication::GroupUp => vec![
            circle(20.0, 10.0, 5.0),
            circle(7.0, 15.0, 4.0),
            circle(33.0, 15.0, 4.0),
            line((12.0, 33.0), (12.0, 23.0)),
            line((12.0, 23.0), (28.0, 23.0)),
            line((28.0, 23.0), (28.0, 33.0)),
            line((2.0, 31.0), (2.0, 25.0)),
            line((2.0, 25.0), (8.0, 25.0)),
            line((32.0, 25.0), (38.0, 25.0)),
            line((38.0, 25.0), (38.0, 31.0)),
        ],
        Communication::UltimateStatus => vec![
            circle(20.0, 20.0, 18.0),
            line((24.0, 7.0), (13.0, 22.0)),
            line((13.0, 22.0), (25.0, 19.0)),
            line((25.0, 19.0), (16.0, 34.0)),
        ],
        Communication::VoiceLine => vec![
            circle(20.0, 15.0, 7.0),
            line((9.0, 17.0), (9.0, 24.0)),
            line((9.0, 24.0), (20.0, 29.0)),
            line((20.0, 29.0), (31.0, 24.0)),
            line((31.0, 24.0), (31.0, 17.0)),
            line((20.0, 29.0), (20.0, 36.0)),
            line((13.0, 36.0), (27.0, 36.0)),
        ],
        Communication::Acknowledge => vec![
            circle(20.0, 20.0, 18.0),
            line((9.0, 20.0), (17.0, 28.0)),
            line((17.0, 28.0), (31.0, 12.0)),
        ],
        Communication::Thanks => {
            let p = [
                (20.0, 35.0),
                (5.0, 20.0),
                (4.0, 12.0),
                (9.0, 7.0),
                (15.0, 7.0),
                (20.0, 13.0),
                (25.0, 7.0),
                (31.0, 7.0),
                (36.0, 12.0),
                (35.0, 20.0),
                (20.0, 35.0),
            ];
            p.windows(2).map(|p| line(p[0], p[1])).collect()
        }
    }
}
impl CommunicationUi {
    pub fn new(ui: &mut UserInterface) -> Self {
        let ctx = &mut ui.build_ctx();
        let dim = panel(ctx, Color::from_rgba(7, 12, 22, 145));
        let sectors =
            std::array::from_fn(|i| vector(ctx, sector(i), Color::from_rgba(30, 43, 61, 245)));
        let outline = vector(
            ctx,
            vec![
                Primitive::WireCircle {
                    center: Vector2::repeat(300.0),
                    radius: 239.0,
                    thickness: 1.5,
                    segments: 128,
                },
                Primitive::Circle {
                    center: Vector2::repeat(300.0),
                    radius: 78.0,
                    segments: 64,
                },
            ],
            Color::from_rgba(17, 27, 41, 245),
        );
        let icons = std::array::from_fn(|i| vector(ctx, icon(Communication::ALL[i]), rgb(CREAM)));
        let labels = std::array::from_fn(|_| centered(ctx, 17.0));
        let title = centered(ctx, 21.0);
        let center = centered(ctx, 18.0);
        let detail = centered(ctx, 13.0);
        let hint = centered(ctx, 15.0);
        let pointer = vector(
            ctx,
            vec![Primitive::Circle {
                center: Vector2::repeat(5.0),
                radius: 4.0,
                segments: 24,
            }],
            rgb([255, 211, 118]),
        );
        let children: Vec<_> = sectors
            .iter()
            .copied()
            .chain([outline])
            .chain(icons)
            .chain(labels)
            .chain([title, center, detail, hint, pointer])
            .collect();
        let root = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_width(600.0)
                .with_height(620.0)
                .with_children(children)
                .with_hit_test_visibility(false),
        )
        .build(ctx)
        .to_base();
        let toast = centered(ctx, 18.0);
        Self {
            dim,
            root,
            title,
            sectors,
            icons,
            labels,
            center,
            detail,
            hint,
            pointer,
            toast,
        }
    }
    pub fn labels(&self) -> impl Iterator<Item = Handle<UiNode>> + '_ {
        self.labels.iter().copied().chain([
            self.title,
            self.center,
            self.detail,
            self.hint,
            self.toast,
        ])
    }
    pub fn render(&self, rt: &Runtime, ui: &UserInterface) {
        let size = ui.screen_size();
        let open = rt.wheel.open && !rt.selecting;
        visible(ui, self.dim, open);
        visible(ui, self.root, open);
        visible(
            ui,
            self.toast,
            rt.communication_notice_time > 0.0 && !rt.selecting,
        );
        place(
            ui,
            self.toast,
            size.x / 2.0 - 270.0,
            size.y / 2.0 + 264.0,
            540.0,
            30.0,
        );
        text(ui, self.toast, &rt.communication_notice);
        if !open {
            return;
        }
        place(ui, self.dim, 0.0, 0.0, size.x, size.y);
        place(
            ui,
            self.root,
            size.x / 2.0 - 300.0,
            size.y / 2.0 - 300.0,
            600.0,
            620.0,
        );
        place(ui, self.title, 0.0, 12.0, 600.0, 32.0);
        text(
            ui,
            self.title,
            if rt.korean_font {
                "팀 커뮤니케이션"
            } else {
                "TEAM COMMUNICATION"
            },
        );
        let selected = rt.wheel.selected();
        for (i, action) in Communication::ALL.into_iter().enumerate() {
            place(ui, self.sectors[i], 0.0, 0.0, 600.0, 600.0);
            let active = selected == Some(action);
            let disabled = action == Communication::VoiceLine
                && !rt.session.world().actors[rt.session.slot()].alive();
            let color = if active {
                if disabled {
                    [76, 82, 92]
                } else {
                    [204, 144, 53]
                }
            } else {
                [30, 43, 61]
            };
            ui.send(
                self.sectors[i],
                WidgetMessage::Foreground(
                    Brush::Solid(Color::from_rgba(color[0], color[1], color[2], 245)).into(),
                ),
            );
            let angle = i as f32 * TAU / 8.0;
            let pos = point(angle, 160.0);
            place(ui, self.icons[i], pos.x - 20.0, pos.y - 38.0, 40.0, 40.0);
            place(ui, self.labels[i], pos.x - 72.0, pos.y + 8.0, 144.0, 50.0);
            text(
                ui,
                self.labels[i],
                if disabled {
                    format!(
                        "{}\n{}",
                        action.label(rt.korean_font),
                        if rt.korean_font {
                            "부활 후 사용"
                        } else {
                            "AFTER RESPAWN"
                        }
                    )
                } else {
                    action.label(rt.korean_font).into()
                },
            );
            let foreground = if disabled { [150, 158, 172] } else { CREAM };
            for h in [self.icons[i], self.labels[i]] {
                ui.send(
                    h,
                    WidgetMessage::Foreground(Brush::Solid(rgb(foreground)).into()),
                );
            }
        }
        place(ui, self.center, 225.0, 274.0, 150.0, 30.0);
        place(ui, self.detail, 220.0, 311.0, 160.0, 40.0);
        text(
            ui,
            self.center,
            selected.map_or(
                if rt.korean_font {
                    "C 누르는 중"
                } else {
                    "HOLD C"
                },
                |a| a.label(rt.korean_font),
            ),
        );
        let detail = match selected {
            Some(Communication::VoiceLine) => {
                if rt.korean_font {
                    "캐릭터 대사 재생\nC를 놓으면 실행".into()
                } else {
                    "CHARACTER VOICE\nRELEASE C".into()
                }
            }
            Some(Communication::UltimateStatus) => format!(
                "{:.0}%",
                rt.session.world().actors[rt.session.slot()]
                    .combat
                    .ultimate
                    .floor()
            ),
            Some(_) => {
                if rt.korean_font {
                    "C를 놓으면 실행".into()
                } else {
                    "RELEASE C".into()
                }
            }
            None => {
                if rt.korean_font {
                    "방향으로 드래그\n중앙은 취소".into()
                } else {
                    "DRAG TO SELECT\nCENTER TO CANCEL".into()
                }
            }
        };
        text(ui, self.detail, detail);
        place(ui, self.hint, 0.0, 558.0, 600.0, 28.0);
        text(
            ui,
            self.hint,
            if rt.korean_font {
                "C 누르기 → 마우스 드래그 → C 놓기  ·  우클릭 / Esc 취소"
            } else {
                "HOLD C → DRAG → RELEASE C  ·  RIGHT CLICK / ESC CANCEL"
            },
        );
        place(
            ui,
            self.pointer,
            295.0 + rt.wheel.x,
            295.0 + rt.wheel.y,
            10.0,
            10.0,
        );
    }
}
