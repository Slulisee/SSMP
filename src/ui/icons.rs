use eframe::egui::{self, Color32, Stroke, Vec2};
#[derive(Clone, Copy)]
pub enum Icon {
    Home,
    Music,
    Album,
    Artist,
    Heart,
    Stats,
    Shuffle,
    Previous,
    Next,
    Play,
    Pause,
    Repeat,
    RepeatOne,
    More,
    Queue,
    Volume,
    Folder,
    Settings,
    Download,
    Lyrics,
    Video,
    Fullscreen,
    Plus,
    Back,
    Clock,
    Search,
    Close,
}
pub fn draw(p: &egui::Painter, rect: egui::Rect, icon: Icon, color: Color32) {
    let r = egui::Rect::from_center_size(
        rect.center(),
        Vec2::splat(rect.width().min(rect.height()) * 0.59),
    );
    let pt = |x: f32, y: f32| egui::pos2(r.left() + r.width() * x, r.top() + r.height() * y);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([pt(a.0, a.1), pt(b.0, b.1)], Stroke::new(1.65_f32, color));
    };
    let path = |points: &[(f32, f32)]| {
        p.add(egui::Shape::line(
            points.iter().map(|v| pt(v.0, v.1)).collect(),
            Stroke::new(1.65_f32, color),
        ));
    };
    match icon {
        Icon::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![pt(0.28, 0.08), pt(0.88, 0.5), pt(0.28, 0.92)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pause => {
            for x in [0.24, 0.64] {
                p.rect_filled(
                    egui::Rect::from_min_max(pt(x, 0.13), pt(x + 0.15, 0.87)),
                    1.,
                    color,
                );
            }
        }
        Icon::Previous | Icon::Next => {
            let flip = matches!(icon, Icon::Previous);
            let x = |x| if flip { 1. - x } else { x };
            p.add(egui::Shape::convex_polygon(
                vec![pt(x(0.13), 0.14), pt(x(0.72), 0.5), pt(x(0.13), 0.86)],
                color,
                Stroke::NONE,
            ));
            line((x(0.86), 0.12), (x(0.86), 0.88));
        }
        Icon::Heart => {
            path(&[
                (0.5, 0.92),
                (0.07, 0.48),
                (0.04, 0.25),
                (0.19, 0.1),
                (0.36, 0.1),
                (0.5, 0.26),
                (0.64, 0.1),
                (0.81, 0.1),
                (0.96, 0.25),
                (0.93, 0.48),
                (0.5, 0.92),
            ]);
        }
        Icon::Home => {
            path(&[(0.04, 0.43), (0.5, 0.04), (0.96, 0.43)]);
            path(&[
                (0.17, 0.4),
                (0.17, 0.93),
                (0.42, 0.93),
                (0.42, 0.63),
                (0.62, 0.63),
                (0.62, 0.93),
                (0.84, 0.93),
                (0.84, 0.4),
            ]);
        }
        Icon::Music => {
            path(&[(0.35, 0.76), (0.35, 0.15), (0.87, 0.03), (0.87, 0.65)]);
            line((0.35, 0.32), (0.87, 0.2));
            p.circle_filled(pt(0.22, 0.8), r.width() * 0.13, color);
            p.circle_filled(pt(0.74, 0.69), r.width() * 0.13, color);
        }
        Icon::Album => {
            p.circle_stroke(r.center(), r.width() * 0.45, Stroke::new(1.65_f32, color));
            p.circle_stroke(r.center(), r.width() * 0.12, Stroke::new(1.65_f32, color));
        }
        Icon::Artist => {
            p.circle_stroke(pt(0.5, 0.27), r.width() * 0.2, Stroke::new(1.65_f32, color));
            path(&[
                (0.1, 0.95),
                (0.13, 0.73),
                (0.31, 0.57),
                (0.69, 0.57),
                (0.87, 0.73),
                (0.9, 0.95),
            ]);
        }
        Icon::Stats => {
            for (x, h) in [(0.14, 0.55), (0.46, 0.15), (0.78, 0.35)] {
                line((x, 0.92), (x, h));
            }
            line((0., 0.97), (0.97, 0.97));
        }
        Icon::Shuffle => {
            path(&[(0.04, 0.18), (0.2, 0.18), (0.72, 0.82), (0.96, 0.82)]);
            path(&[(0.04, 0.82), (0.2, 0.82), (0.72, 0.18), (0.96, 0.18)]);
            path(&[(0.76, 0.02), (0.96, 0.18), (0.76, 0.34)]);
            path(&[(0.76, 0.66), (0.96, 0.82), (0.76, 0.98)]);
        }
        Icon::Repeat | Icon::RepeatOne => {
            path(&[(0.12, 0.56), (0.12, 0.23), (0.9, 0.23), (0.72, 0.05)]);
            path(&[(0.9, 0.42), (0.9, 0.78), (0.1, 0.78), (0.28, 0.96)]);
            if matches!(icon, Icon::RepeatOne) {
                path(&[(0.43, 0.44), (0.52, 0.37), (0.52, 0.65)]);
                line((0.43, 0.65), (0.61, 0.65));
            }
        }
        Icon::More => {
            for x in [0.16, 0.5, 0.84] {
                p.circle_filled(pt(x, 0.5), r.width() * 0.07, color);
            }
        }
        Icon::Queue => {
            for y in [0.2, 0.5, 0.8] {
                p.circle_filled(pt(0.08, y), r.width() * 0.04, color);
                line((0.28, y), (0.95, y));
            }
        }
        Icon::Lyrics => {
            path(&[
                (0.05, 0.1),
                (0.95, 0.1),
                (0.95, 0.75),
                (0.5, 0.75),
                (0.25, 0.95),
                (0.25, 0.75),
                (0.05, 0.75),
                (0.05, 0.1),
            ]);
            line((0.23, 0.32), (0.76, 0.32));
            line((0.23, 0.53), (0.64, 0.53));
        }
        Icon::Volume => {
            path(&[
                (0.04, 0.37),
                (0.25, 0.37),
                (0.48, 0.14),
                (0.48, 0.86),
                (0.25, 0.63),
                (0.04, 0.63),
                (0.04, 0.37),
            ]);
            path(&[(0.67, 0.27), (0.78, 0.5), (0.67, 0.73)]);
            path(&[(0.84, 0.1), (0.98, 0.5), (0.84, 0.9)]);
        }
        Icon::Folder => {
            path(&[
                (0.03, 0.9),
                (0.03, 0.14),
                (0.4, 0.14),
                (0.5, 0.31),
                (0.96, 0.31),
                (0.96, 0.9),
                (0.03, 0.9),
            ]);
        }
        Icon::Settings => {
            p.circle_stroke(r.center(), r.width() * 0.16, Stroke::new(1.65_f32, color));
            p.circle_stroke(r.center(), r.width() * 0.37, Stroke::new(1.65_f32, color));
            for a in 0..8 {
                let a = a as f32 * std::f32::consts::TAU / 8.;
                line(
                    (0.5 + a.cos() * 0.36, 0.5 + a.sin() * 0.36),
                    (0.5 + a.cos() * 0.49, 0.5 + a.sin() * 0.49),
                );
            }
        }
        Icon::Download => {
            line((0.5, 0.02), (0.5, 0.69));
            path(&[(0.25, 0.45), (0.5, 0.7), (0.75, 0.45)]);
            path(&[(0.05, 0.73), (0.05, 0.96), (0.95, 0.96), (0.95, 0.73)]);
        }
        Icon::Video => {
            p.rect_stroke(
                egui::Rect::from_min_max(pt(0.03, 0.18), pt(0.69, 0.82)),
                2.,
                Stroke::new(1.65_f32, color),
                egui::StrokeKind::Inside,
            );
            path(&[(0.69, 0.38), (0.97, 0.21), (0.97, 0.79), (0.69, 0.62)]);
        }
        Icon::Fullscreen => {
            for (x, y, dx, dy) in [
                (0.05, 0.05, 1., 1.),
                (0.95, 0.05, -1., 1.),
                (0.05, 0.95, 1., -1.),
                (0.95, 0.95, -1., -1.),
            ] {
                line((x, y), (x + dx * 0.25, y));
                line((x, y), (x, y + dy * 0.25));
            }
        }
        Icon::Plus => {
            line((0.5, 0.1), (0.5, 0.9));
            line((0.1, 0.5), (0.9, 0.5));
        }
        Icon::Back => {
            path(&[(0.65, 0.1), (0.25, 0.5), (0.65, 0.9)]);
        }
        Icon::Clock => {
            p.circle_stroke(r.center(), r.width() * 0.45, Stroke::new(1.65_f32, color));
            path(&[(0.5, 0.21), (0.5, 0.5), (0.72, 0.6)]);
        }
        Icon::Search => {
            p.circle_stroke(pt(0.4, 0.4), r.width() * 0.32, Stroke::new(1.65_f32, color));
            line((0.64, 0.64), (0.95, 0.95));
        }
        Icon::Close => {
            line((0.2, 0.2), (0.8, 0.8));
            line((0.2, 0.8), (0.8, 0.2));
        }
    }
}
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    active: bool,
    size: f32,
) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());
    let hovered = ui.ctx().animate_bool(r.id.with("hover"), r.hovered());
    let primary = matches!(icon, Icon::Play | Icon::Pause) && size >= 40.;
    if primary {
        ui.painter().circle_filled(
            rect.center(),
            size / 2.,
            ui.visuals().selection.stroke.color,
        );
    }
    if !primary && (active || hovered > 0.01) {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same((size / 2.) as u8),
            ui.visuals()
                .selection
                .bg_fill
                .gamma_multiply(if active { 0.9 } else { hovered * 0.8 }),
        );
    }
    draw(
        ui.painter(),
        rect,
        icon,
        if primary {
            let c = ui.visuals().selection.stroke.color;
            if c.r() as u32 + c.g() as u32 + c.b() as u32 > 490 {
                Color32::BLACK
            } else {
                Color32::WHITE
            }
        } else if active {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        },
    );
    r.on_hover_text(label)
}
