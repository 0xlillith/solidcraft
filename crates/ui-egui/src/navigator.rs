//! Original orientation widgets. Product names describe familiar visual conventions;
//! no external code, artwork or product assets are used.
use egui::{Color32, FontId, Painter, Pos2, Shape, Stroke, vec2};
use serde::{Deserialize, Serialize};
use solvecraft_engine::{geom::Vec3, render::Camera};

use crate::theme::Tokens;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[default]
    Axes,
    Classic,
    Fusion,
    Inventor,
    Blender,
    Solidworks,
    Nx,
}

impl Style {
    pub const ALL: [Self; 7] = [Self::Axes, Self::Classic, Self::Fusion, Self::Inventor, Self::Blender, Self::Solidworks, Self::Nx];

    pub fn label(self) -> &'static str {
        match self {
            Self::Axes => "SolidCraft axes (current)",
            Self::Classic => "SolidCraft cube (classic)",
            Self::Fusion => "Fusion-inspired cube",
            Self::Inventor => "Inventor-inspired cube",
            Self::Blender => "Blender-inspired gizmo",
            Self::Solidworks => "SOLIDWORKS-inspired triad",
            Self::Nx => "NX-inspired cube",
        }
    }

    pub fn is_cube(self) -> bool {
        matches!(self, Self::Classic | Self::Fusion | Self::Inventor | Self::Nx)
    }
}

/// A projected tile owns both its drawn highlight and its exact hit polygon.
pub(crate) struct Tile {
    pub points: Vec<Pos2>,
    pub direction: Vec3,
    pub face: Vec3,
    pub label: &'static str,
    pub centre: Pos2,
}

impl Tile {
    pub fn contains(&self, p: Pos2) -> bool {
        let mut positive = false;
        let mut negative = false;
        for (a, b) in self.points.iter().zip(self.points.iter().cycle().skip(1)) {
            let cross = (*b - *a).rot90().dot(p - *a);
            positive |= cross > 0.000001;
            negative |= cross < -0.000001;
        }
        !(positive && negative)
    }
}

pub(crate) fn tiles(cam: Camera, centre: Pos2, style: Style) -> Vec<Tile> {
    if matches!(style, Style::Classic | Style::Inventor) {
        return beveled_tiles(cam, centre);
    }
    let (right, up, back) = cam.basis();
    let project = |p: Vec3| centre + vec2((p.dot(right) * 25.0) as f32, -(p.dot(up) * 25.0) as f32);
    let mut out = Vec::new();
    // Named faces follow the modelling up-axis; projected geometry stays in world space.
    for (normal, a, b, label) in [
        (Vec3::X, Vec3::Y, Vec3::Z, "RIGHT"),
        (-Vec3::X, Vec3::Y, Vec3::Z, "LEFT"),
        (-Vec3::Y, Vec3::X, Vec3::Z, "FRONT"),
        (Vec3::Y, Vec3::X, Vec3::Z, "BACK"),
        (Vec3::Z, Vec3::X, Vec3::Y, "TOP"),
        (-Vec3::Z, Vec3::X, Vec3::Y, "BOTTOM"),
    ] {
        let normal = cam.from_frame(normal);
        let a = cam.from_frame(a);
        let b = cam.from_frame(b);
        if normal.dot(back) <= 1e-3 {
            continue;
        }
        // Broad face centre, narrow edge strips and corner squares (all 26 directions).
        for (i, (lo_a, hi_a)) in [(-1.0, -0.58), (-0.58, 0.58), (0.58, 1.0)].into_iter().enumerate() {
            for (j, (lo_b, hi_b)) in [(-1.0, -0.58), (-0.58, 0.58), (0.58, 1.0)].into_iter().enumerate() {
                let direction = normal + a * (i as f64 - 1.0) + b * (j as f64 - 1.0);
                let points =
                    [(lo_a, lo_b), (hi_a, lo_b), (hi_a, hi_b), (lo_a, hi_b)].into_iter().map(|(x, y)| project(normal + a * x + b * y)).collect();
                out.push(Tile {
                    points,
                    direction,
                    face: normal,
                    label,
                    centre: project(normal + a * ((lo_a + hi_a) * 0.5) + b * ((lo_b + hi_b) * 0.5)),
                });
            }
        }
    }
    out
}

/// A bevelled cube: six square faces, twelve edge quads and eight corner triangles.
/// Each surface is a real target, so the classic cube also gains edge views.
fn beveled_tiles(cam: Camera, centre: Pos2) -> Vec<Tile> {
    let (right, up, back) = cam.basis();
    let mut out = Vec::new();
    for x in [-1.0_f64, 0.0, 1.0] {
        for y in [-1.0_f64, 0.0, 1.0] {
            for z in [-1.0_f64, 0.0, 1.0] {
                let d = Vec3::new(x, y, z);
                let Some(n) = d.normalized() else { continue };
                let face = cam.from_frame(n);
                if face.dot(back) <= 1e-3 {
                    continue;
                }
                let active: Vec<_> =
                    [(Vec3::X, x), (Vec3::Y, y), (Vec3::Z, z)].into_iter().filter(|(_, sign)| *sign != 0.0).map(|(a, sign)| a * sign).collect();
                let free: Vec<_> = [(Vec3::X, x), (Vec3::Y, y), (Vec3::Z, z)].into_iter().filter(|(_, sign)| *sign == 0.0).map(|(a, _)| a).collect();
                let bevel = 0.68;
                let vertices = match (active.as_slice(), free.as_slice()) {
                    ([n], [a, b]) => {
                        vec![*n - *a * bevel - *b * bevel, *n + *a * bevel - *b * bevel, *n + *a * bevel + *b * bevel, *n - *a * bevel + *b * bevel]
                    }
                    ([a, b], [c]) => {
                        vec![*a + *b * bevel - *c * bevel, *a * bevel + *b - *c * bevel, *a * bevel + *b + *c * bevel, *a + *b * bevel + *c * bevel]
                    }
                    ([a, b, c], []) => vec![*a + (*b + *c) * bevel, *b + (*a + *c) * bevel, *c + (*a + *b) * bevel],
                    _ => Vec::new(),
                };
                let points: Vec<_> = vertices
                    .into_iter()
                    .map(|p| {
                        let p = cam.from_frame(p);
                        centre + vec2((p.dot(right) * 25.0) as f32, -(p.dot(up) * 25.0) as f32)
                    })
                    .collect();
                let count = points.len().max(1) as f32;
                let at = egui::pos2(points.iter().map(|p| p.x).sum::<f32>() / count, points.iter().map(|p| p.y).sum::<f32>() / count);
                let label = match (x, y, z) {
                    (1.0, 0.0, 0.0) => "RIGHT",
                    (-1.0, 0.0, 0.0) => "LEFT",
                    (0.0, -1.0, 0.0) => "FRONT",
                    (0.0, 1.0, 0.0) => "BACK",
                    (0.0, 0.0, 1.0) => "TOP",
                    (0.0, 0.0, -1.0) => "BOTTOM",
                    _ => "",
                };
                out.push(Tile { points, direction: cam.from_frame(d), face, label, centre: at });
            }
        }
    }
    out
}

pub(crate) fn paint_cube(painter: &Painter, cam: Camera, centre: Pos2, style: Style, tiles: &[Tile], hovered: Option<usize>) {
    let t = Tokens::get();
    if style == Style::Fusion {
        // Original compass ring; deliberately does not reproduce another product's artwork.
        painter.circle_stroke(centre, 51.0, Stroke::new(1.5, t.text_dim));
        for offset in [vec2(0.0, -51.0), vec2(51.0, 0.0), vec2(0.0, 51.0), vec2(-51.0, 0.0)] {
            painter.line_segment([centre + offset * 0.9, centre + offset], Stroke::new(2.0, t.accent));
        }
    }
    for (i, tile) in tiles.iter().enumerate() {
        let fill = if hovered == Some(i) {
            t.accent_soft
        } else if style == Style::Nx {
            t.overlay
        } else if style == Style::Inventor {
            t.panel_header
        } else {
            t.panel.gamma_multiply((0.75 + 0.25 * tile.face.dot(cam.back())) as f32)
        };
        painter.extend(crate::viewport::projected_polygon(&tile.points, fill.to_opaque(), Stroke::NONE));
        if hovered == Some(i) {
            painter.extend(crate::viewport::projected_polygon(&tile.points, t.accent_soft, Stroke::new(1.5, t.accent)));
        }
    }
    for tile in tiles {
        if !tile.label.is_empty() && tile.direction == tile.face {
            paint_label(painter, cam, centre, tile, t.text, if matches!(style, Style::Classic | Style::Inventor) { 30.0 } else { 42.0 });
        }
    }
    if matches!(style, Style::Classic | Style::Inventor) {
        for tile in tiles {
            painter.extend(crate::viewport::projected_polygon(&tile.points, Color32::TRANSPARENT, Stroke::new(1.0, t.text_dim)));
        }
        return;
    }
    // Outline each visible face, without seams between its nine hit targets.
    let (r, u, _) = cam.basis();
    for chunk in tiles.as_chunks::<9>().0 {
        if let Some(tile) = chunk.first() {
            let face = tile.face;
            let a = if face.dot(Vec3::X).abs() > 0.5 { Vec3::Y } else { Vec3::X };
            let b = face.cross(a);
            let points = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
                .into_iter()
                .map(|(x, y)| {
                    let p = face + a * x + b * y;
                    centre + vec2((p.dot(r) * 25.0) as f32, -(p.dot(u) * 25.0) as f32)
                })
                .collect();
            let points: Vec<_> = points;
            painter.extend(crate::viewport::projected_polygon(
                &points,
                Color32::TRANSPARENT,
                Stroke::new(if style == Style::Nx { 2.0 } else { 1.0 }, t.text_dim),
            ));
        }
    }
}

// Adapted from SolidCraft's historical view-cube label projection (see ATTRIBUTION.md).
fn paint_label(painter: &Painter, cam: Camera, centre: Pos2, tile: &Tile, ink: Color32, width: f32) {
    if tile.face.dot(cam.back()) < 0.08 {
        return;
    }
    let ctx = painter.ctx();
    let galley = ctx.fonts_mut(|f| f.layout_no_wrap(tile.label.into(), FontId::proportional(11.0), ink));
    let options = egui::epaint::TessellationOptions { round_text_to_pixels: false, ..Default::default() };
    let mut tess = egui::epaint::Tessellator::new(ctx.pixels_per_point(), options, ctx.fonts(|f| f.font_image_size()), vec![]);
    let mut mesh = egui::Mesh::default();
    let width_scale = (width / galley.size().x.max(1.0)).min(1.0);
    let at = (-galley.size() / 2.0).to_pos2();
    tess.tessellate_text(&egui::epaint::TextShape::new(at, galley, ink), &mut mesh);
    let n = cam.to_frame(tile.face);
    let (right, up) = if n.z.abs() > 0.5 { (Vec3::X, Vec3::Y * n.z.signum()) } else { (Vec3::Z.cross(n), Vec3::Z) };
    let (r, u, _) = cam.basis();
    for vertex in &mut mesh.vertices {
        let p = tile.face + cam.from_frame(right) * (f64::from(vertex.pos.x * width_scale) / 25.0)
            - cam.from_frame(up) * (f64::from(vertex.pos.y * width_scale) / 25.0);
        vertex.pos = centre + vec2((p.dot(r) * 25.0) as f32, -(p.dot(u) * 25.0) as f32);
    }
    painter.add(Shape::mesh(mesh));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use solvecraft_engine::render::UpAxis;

    #[test]
    fn navigator_selector_is_reachable_in_preferences() {
        let mut h = crate::scenario::Harness::new();
        h.call("ui.prefs", json!({"open":"Navigation"}));
        h.step(&json!({"widget":Style::Axes.label()})).unwrap();
        h.step(&json!({"widget":Style::Nx.label()})).unwrap();
        assert_eq!(h.app.preferences.navigator_style, Style::Nx);
        h.call("ui.prefs", json!({"button":"ok"}));
        assert!(crate::scenario::handle_at("view_cube:1,-1,1").is_some());
    }

    #[test]
    fn all_navigators_drag_once_and_respect_floating_dialog_input() {
        for style in Style::ALL {
            let mut h = crate::scenario::Harness::new();
            h.app.preferences.navigator_style = style;
            h.app.preferences.look_at_anim = false;
            h.call("ui.resize", json!({"width":800,"height":700}));
            h.call("ui.view", json!({"view":"front"}));
            let name = if style.is_cube() { "view_cube:0,-1,0" } else { "axis_view:-y" };
            let p = crate::scenario::handle_at(name).unwrap();
            let before = h.app.cam;
            h.call("ui.drag", json!({"x0":p.x,"y0":p.y,"x1":p.x-32.0,"y1":p.y,"steps":8}));
            h.frames(120);
            assert!((h.app.cam.yaw - before.yaw + 32.0 * 0.008).abs() < 1e-5, "{style:?}");
            assert_eq!(h.app.cam.target, before.target);
            assert_eq!(h.app.cam.distance, before.distance);
            assert!(h.app.viewport.axis_drag.is_none());
            h.call("ui.prefs", json!({"open":"Navigation"}));
            let dialog = h.ctx.memory(|m| m.area_rect(egui::Id::new("sc_prefs"))).unwrap();
            assert!(dialog.contains(p));
            let before = h.app.cam;
            h.call("ui.click", json!({"x":p.x,"y":p.y}));
            h.call("ui.drag", json!({"x0":p.x,"y0":p.y,"x1":p.x-20.0,"y1":p.y,"steps":8}));
            assert_eq!(h.app.cam.back(), before.back(), "{style:?}: dialog owns its clicks and drags");
            assert!(h.app.viewport.axis_drag.is_none());
        }
    }

    #[test]
    fn navigator_preferences_persist_and_cancel_without_changing_mouse_mapping() {
        let mut h = crate::scenario::Harness::new();
        assert_eq!(serde_json::from_value::<crate::prefs::Prefs>(json!({})).unwrap().navigator_style, Style::Axes);
        for style in Style::ALL {
            h.call("ui.prefs", json!({"open":"Navigation", "navigator_style":style, "button":"apply"}));
            assert_eq!(h.app.preferences.nav, "fusion");
            assert_eq!(h.app.preferences.navigator_style, style);
            let mut restored = crate::SolveApp::new(Default::default(), Default::default());
            restored.load_prefs(&h.app.prefs());
            assert_eq!(restored.preferences.navigator_style, style);
            h.call("ui.prefs", json!({"navigator_style":"axes", "button":"cancel"}));
            assert_eq!(h.app.preferences.navigator_style, style);
        }
        let before = h.app.preferences.clone();
        assert!(crate::prefs::set(&mut h.app, &json!({"navigator_style":"unknown"})).is_err());
        assert_eq!(h.app.preferences, before);
        h.call("ui.prefs", json!({"open":"Navigation", "button":"defaults"}));
        assert_eq!(h.app.preferences.navigator_style, Style::Axes);
    }

    #[test]
    fn cube_faces_edges_and_corners_click_to_all_26_views_in_both_up_frames() {
        let mut h = crate::scenario::Harness::new();
        h.app.preferences.look_at_anim = false;
        for up in [UpAxis::Z, UpAxis::Y] {
            h.app.set_up_axis(up);
            h.frames(2);
            for style in Style::ALL.into_iter().filter(|s| s.is_cube()) {
                h.app.preferences.navigator_style = style;
                for x in [-1.0, 0.0, 1.0] {
                    for y in [-1.0, 0.0, 1.0] {
                        for z in [-1.0, 0.0, 1.0] {
                            let frame = Vec3::new(x, y, z);
                            let Some(unit) = frame.normalized() else { continue };
                            let dir = h.app.cam.from_frame(unit);
                            // Offset the eye to make each requested surface visible, without starting aligned.
                            h.app.cam = h.app.cam.looking_from(dir + h.app.cam.from_frame(Vec3::new(0.13, 0.09, 0.07)));
                            h.frames(2);
                            let target = format!("view_cube:{x:.0},{y:.0},{z:.0}");
                            h.step(&json!({"click":{"handle":target}})).unwrap();
                            assert!(h.app.cam.back().dist(dir) < 1e-6, "{style:?} {up:?} {frame:?}: {:?}", h.app.cam.back());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn projected_tiles_own_their_centres_and_stay_inside_reserved_space() {
        let centre = egui::pos2(100.0, 100.0);
        for direction in crate::viewport::orbit_directions() {
            let cam = Camera::default().looking_from(direction);
            let tiles = tiles(cam, centre, Style::Fusion);
            for (i, tile) in tiles.iter().enumerate() {
                assert!(tile.contains(tile.centre));
                assert_eq!(tiles.iter().position(|t| t.contains(tile.centre)), Some(i));
                assert!(!tile.contains(centre + vec2(100.0, 100.0)));
                for point in &tile.points {
                    assert!(point.distance(centre) <= 44.0);
                }
                let shapes = crate::viewport::projected_polygon(&tile.points, Color32::GRAY, Stroke::new(2.0, Color32::BLACK));
                for point in crate::viewport::drawn_points(shapes) {
                    assert!(point.distance(centre) <= 47.0, "near-edge-on faces must not produce spikes");
                }
            }
        }
    }
}
