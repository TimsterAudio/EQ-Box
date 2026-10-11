//! Drawing a spectrum in a chosen colour scheme and style. Shared by EQ
//! Box's analyser (behind the response graph) and SP-10, the analyser on its own.

use super::theme::{lerp_colour, LCD_INK};
use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{Color32, Painter, Pos2, Shape, Stroke};
use serde::{Deserialize, Serialize};

/// The analyser's colours. Stored by name in saved settings, so never
/// rename a variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scheme {
    /// The display red, like the rest of the panel.
    #[default]
    Display,
    Amber,
    Green,
    Ice,
    White,
    /// A rainbow from the bass (red) to the treble (violet).
    Spectrum,
    /// Colour by level: dark red when quiet through orange to yellow-white.
    Heat,
}

impl Scheme {
    pub const ALL: [Scheme; 7] = [
        Scheme::Display,
        Scheme::Amber,
        Scheme::Green,
        Scheme::Ice,
        Scheme::White,
        Scheme::Spectrum,
        Scheme::Heat,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Scheme::Display => "RED",
            Scheme::Amber => "AMBER",
            Scheme::Green => "GREEN",
            Scheme::Ice => "ICE",
            Scheme::White => "WHITE",
            Scheme::Spectrum => "RAINBOW",
            Scheme::Heat => "HEAT",
        }
    }

    pub fn next(self) -> Scheme {
        let i = Scheme::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Scheme::ALL[(i + 1) % Scheme::ALL.len()]
    }

    /// The colour at horizontal position `x` (0 = left, 1 = right) and
    /// level `level` (0 = bottom, 1 = top).
    pub fn colour(self, x: f32, level: f32) -> Color32 {
        match self {
            Scheme::Display => LCD_INK,
            Scheme::Amber => Color32::from_rgb(0xff, 0xa8, 0x2e),
            Scheme::Green => Color32::from_rgb(0x5c, 0xf0, 0x7a),
            Scheme::Ice => Color32::from_rgb(0x6c, 0xc8, 0xff),
            Scheme::White => Color32::from_rgb(0xec, 0xe8, 0xdc),
            Scheme::Spectrum => rainbow(x),
            Scheme::Heat => heat(level),
        }
    }
}

/// How the spectrum is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Look {
    /// A filled area with a line on top.
    #[default]
    FillAndLine,
    LineOnly,
    FillOnly,
}

impl Look {
    pub const ALL: [Look; 3] = [Look::FillAndLine, Look::LineOnly, Look::FillOnly];

    pub fn name(self) -> &'static str {
        match self {
            Look::FillAndLine => "FILL+LINE",
            Look::LineOnly => "LINE",
            Look::FillOnly => "FILL",
        }
    }

    pub fn next(self) -> Look {
        let i = Look::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Look::ALL[(i + 1) % Look::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub scheme: Scheme,
    pub fill: Look,
    /// Overall strength: low behind the EQ curve, full in SP-10.
    pub strength: f32,
}

/// Red through yellow, green, blue to violet across `x` in 0..1.
fn rainbow(x: f32) -> Color32 {
    let stops = [
        (0.0, Color32::from_rgb(0xff, 0x40, 0x30)),
        (0.2, Color32::from_rgb(0xff, 0x9a, 0x20)),
        (0.4, Color32::from_rgb(0xf0, 0xe0, 0x40)),
        (0.6, Color32::from_rgb(0x50, 0xe8, 0x70)),
        (0.8, Color32::from_rgb(0x40, 0xa8, 0xff)),
        (1.0, Color32::from_rgb(0xb0, 0x70, 0xff)),
    ];
    ramp(&stops, x)
}

fn heat(level: f32) -> Color32 {
    let stops = [
        (0.0, Color32::from_rgb(0x70, 0x10, 0x08)),
        (0.45, Color32::from_rgb(0xe0, 0x30, 0x18)),
        (0.75, Color32::from_rgb(0xff, 0x9a, 0x20)),
        (1.0, Color32::from_rgb(0xff, 0xf0, 0xb0)),
    ];
    ramp(&stops, level)
}

fn ramp(stops: &[(f32, Color32)], t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        if t <= t1 {
            return lerp_colour(c0, c1, (t - t0) / (t1 - t0).max(1e-6));
        }
    }
    stops[stops.len() - 1].1
}

/// Draw a spectrum curve. `pts` run left to right; `level(i)` is how high
/// point `i` is in 0..1 (for the Heat scheme); `left`/`right` are the
/// plot's edges and `base_y` is where the fill fades out.
pub fn draw(
    p: &Painter,
    pts: &[Pos2],
    levels: &[f32],
    left: f32,
    right: f32,
    base_y: f32,
    style: Style,
) {
    if pts.len() < 2 {
        return;
    }
    let width = (right - left).max(1.0);
    let col = |i: usize| style.scheme.colour((pts[i].x - left) / width, levels[i]);
    if style.fill != Look::LineOnly {
        let top_alpha = if style.fill == Look::FillOnly {
            0.55
        } else {
            0.32
        } * style.strength;
        let mut mesh = Mesh::default();
        for (i, pt) in pts.iter().enumerate() {
            let c = col(i);
            mesh.vertices.push(Vertex {
                pos: *pt,
                uv: WHITE_UV,
                color: c.gamma_multiply(top_alpha),
            });
            mesh.vertices.push(Vertex {
                pos: Pos2::new(pt.x, base_y),
                uv: WHITE_UV,
                color: c.gamma_multiply(top_alpha * 0.15),
            });
        }
        for i in 0..pts.len() as u32 - 1 {
            let a = i * 2;
            mesh.add_triangle(a, a + 1, a + 2);
            mesh.add_triangle(a + 1, a + 3, a + 2);
        }
        p.add(Shape::mesh(mesh));
    }
    if style.fill != Look::FillOnly {
        let alpha = 0.9 * style.strength;
        let single = !matches!(style.scheme, Scheme::Spectrum | Scheme::Heat);
        if single {
            p.add(Shape::line(
                pts.to_vec(),
                Stroke::new(1.2, col(0).gamma_multiply(alpha)),
            ));
        } else {
            for i in 0..pts.len() - 1 {
                p.line_segment(
                    [pts[i], pts[i + 1]],
                    Stroke::new(1.2, col(i).gamma_multiply(alpha)),
                );
            }
        }
    }
}
