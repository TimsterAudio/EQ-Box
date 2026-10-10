//! Panel hardware drawn with egui's painter: knurled knobs, faders, LED keys,
//! red readouts, the LED display and the outlined sections.

use super::theme::*;
use egui::epaint::{Mesh, TextShape};
use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align2, Color32, CornerRadius, FontId, Id, Key, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2,
};
use std::f32::consts::PI;

/// How a control maps its value onto its travel.
#[derive(Clone, Copy)]
pub struct Spec {
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub log: bool,
    /// Value grid; 0 means 3 significant figures for log controls.
    pub step: f32,
}

impl Spec {
    pub fn norm(&self, v: f32) -> f32 {
        let t = if self.log {
            (v.max(self.min).ln() - self.min.ln()) / (self.max.ln() - self.min.ln())
        } else {
            (v - self.min) / (self.max - self.min)
        };
        t.clamp(0.0, 1.0)
    }

    pub fn denorm(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        if self.log {
            (self.min.ln() + t * (self.max.ln() - self.min.ln())).exp()
        } else {
            self.min + t * (self.max - self.min)
        }
    }

    pub fn snap(&self, v: f32) -> f32 {
        let v = if self.step > 0.0 {
            (v / self.step).round() * self.step
        } else if self.log && v > 0.0 {
            let mag = 10f32.powf(v.log10().floor() - 2.0);
            (v / mag).round() * mag
        } else {
            v
        };
        v.clamp(self.min, self.max)
    }
}

// ---------------------------------------------------------------- text

pub fn text(
    p: &Painter,
    pos: Pos2,
    align: Align2,
    s: &str,
    font: FontId,
    color: Color32,
    spacing: f32,
) -> Rect {
    let mut job = LayoutJob::default();
    job.append(
        s,
        0.0,
        TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    let galley = p.layout_job(job);
    // Letter spacing also follows the last glyph; don't let it skew centring.
    let size = galley.size() - Vec2::new(spacing, 0.0);
    let rect = align.anchor_size(pos, size);
    p.galley(rect.min, galley, color);
    rect
}

/// Red display text with a soft glow.
pub fn glow_text(
    p: &Painter,
    pos: Pos2,
    align: Align2,
    s: &str,
    font: FontId,
    color: Color32,
    spacing: f32,
) -> Rect {
    for d in [
        Vec2::new(-1.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(0.0, 1.0),
    ] {
        text(
            p,
            pos + d,
            align,
            s,
            font.clone(),
            LCD_GLOW.gamma_multiply(0.45),
            spacing,
        );
    }
    text(p, pos, align, s, font, color, spacing)
}

/// Panel legend: uppercase, letter-spaced Barlow Semi Condensed.
pub fn legend(p: &Painter, pos: Pos2, align: Align2, s: &str, th: &Theme) -> Rect {
    text(
        p,
        pos,
        align,
        &s.to_uppercase(),
        font(F_LEGEND, 11.0),
        th.legend,
        1.3,
    )
}

/// Text slanted like the Jupiter badge.
pub fn slanted_text(
    ui: &Ui,
    pos: Pos2,
    s: &str,
    font_id: FontId,
    color: Color32,
    spacing: f32,
    degrees: f32,
) {
    let p = ui.painter();
    let mut job = LayoutJob::default();
    job.append(
        s,
        0.0,
        TextFormat {
            font_id,
            color,
            extra_letter_spacing: spacing,
            ..Default::default()
        },
    );
    let galley = p.layout_job(job);
    let ctx = ui.ctx();
    let tex_size = ctx.fonts(|f| f.font_image_size());
    let mut tess = egui::epaint::Tessellator::new(
        ctx.pixels_per_point(),
        ctx.tessellation_options(|o| *o),
        tex_size,
        Vec::new(),
    );
    let baseline = pos.y + galley.size().y * 0.8;
    let mut mesh = Mesh::default();
    tess.tessellate_text(&TextShape::new(pos, galley, color), &mut mesh);
    let k = degrees.to_radians().tan();
    for v in &mut mesh.vertices {
        v.pos.x += (baseline - v.pos.y) * k;
    }
    p.add(Shape::mesh(mesh));
}

// ---------------------------------------------------------------- frames

/// An outlined section with its title set into the top edge.
pub fn section(p: &Painter, rect: Rect, title: &str, th: &Theme, highlight: bool) {
    let stroke = if highlight {
        Stroke::new(1.5, FOCUS)
    } else {
        Stroke::new(1.0, th.frame)
    };
    p.rect_stroke(rect, CornerRadius::same(3), stroke, StrokeKind::Middle);
    if !title.is_empty() {
        let mut job = LayoutJob::default();
        job.append(
            &title.to_uppercase(),
            0.0,
            TextFormat {
                font_id: font(F_LEGEND_BOLD, 12.5),
                color: th.legend,
                extra_letter_spacing: 2.6,
                ..Default::default()
            },
        );
        let g = p.layout_job(job);
        let pos = Pos2::new(rect.left() + 14.0, rect.top() - g.size().y / 2.0);
        let bg = Rect::from_min_size(pos - Vec2::new(6.0, 0.0), g.size() + Vec2::new(8.0, 0.0));
        p.rect_filled(bg, 0, th.panel);
        p.galley(pos, g, th.legend);
    }
}

/// The main LED display: two lines, each with a left and right part.
pub fn lcd(p: &Painter, rect: Rect, lines: &[(String, String); 2]) {
    p.rect_filled(
        rect.expand(3.0),
        CornerRadius::same(4),
        Color32::from_white_alpha(18),
    );
    p.rect_filled(rect.expand(2.0), CornerRadius::same(4), Color32::BLACK);
    p.rect_filled(rect, CornerRadius::same(3), LCD_BG);
    let f = font(F_MONO, 15.0);
    for (i, (l, r)) in lines.iter().enumerate() {
        let y = rect.top() + 9.0 + i as f32 * 20.0;
        let col = if i == 0 {
            LCD_INK
        } else {
            LCD_INK.gamma_multiply(0.85)
        };
        let rr = glow_text(
            p,
            Pos2::new(rect.right() - 12.0, y),
            Align2::RIGHT_TOP,
            &r.to_uppercase(),
            f.clone(),
            col,
            0.9,
        );
        // Keep the left part clear of the right part.
        let clip = Rect::from_min_max(rect.min, Pos2::new(rr.left() - 10.0, rect.bottom()));
        glow_text(
            &p.with_clip_rect(clip),
            Pos2::new(rect.left() + 12.0, y),
            Align2::LEFT_TOP,
            &l.to_uppercase(),
            f.clone(),
            col,
            0.9,
        );
    }
}

/// A small red value readout, centred under a control.
pub fn readout(p: &Painter, top_centre: Pos2, s: &str, min_w: f32) {
    let f = font(F_MONO, 12.0);
    let w = p
        .layout_no_wrap(s.to_uppercase(), f.clone(), LCD_INK)
        .size()
        .x
        + 12.0;
    let rect = Rect::from_center_size(
        top_centre + Vec2::new(0.0, 8.0),
        Vec2::new(w.max(min_w), 16.0),
    );
    p.rect_filled(rect.expand(1.0), CornerRadius::same(2), Color32::BLACK);
    p.rect_filled(rect, CornerRadius::same(2), LCD_BG);
    glow_text(
        p,
        rect.center(),
        Align2::CENTER_CENTER,
        &s.to_uppercase(),
        f,
        LCD_INK,
        0.5,
    );
}

/// Stereo LED bar meter (peaks in dBFS).
pub fn meter(p: &Painter, rect: Rect, db: [f32; 2]) {
    p.rect_filled(rect.expand(2.0), CornerRadius::same(2), Color32::BLACK);
    p.rect_filled(
        rect,
        CornerRadius::same(2),
        Color32::from_rgb(0x0b, 0x10, 0x0b),
    );
    const SEGS: usize = 30;
    let gap = 2.0;
    let seg_w = (rect.width() - 6.0 - gap * (SEGS - 1) as f32) / SEGS as f32;
    let row_h = (rect.height() - 6.0 - gap) / 2.0;
    for (row, level) in db.iter().enumerate() {
        let lit = ((level + 48.0) / 48.0 * SEGS as f32).ceil() as i32;
        for s in 0..SEGS {
            let seg_db = -48.0 + 48.0 * (s as f32 + 1.0) / SEGS as f32;
            let on = (s as i32) < lit;
            let base = if seg_db > -3.0 {
                Color32::from_rgb(0xff, 0x3a, 0x24)
            } else if seg_db > -12.0 {
                Color32::from_rgb(0xef, 0xc2, 0x3b)
            } else {
                Color32::from_rgb(0x4f, 0xd0, 0x6a)
            };
            let col = if on { base } else { base.gamma_multiply(0.13) };
            let x = rect.left() + 3.0 + s as f32 * (seg_w + gap);
            let y = rect.top() + 3.0 + row as f32 * (row_h + gap);
            p.rect_filled(
                Rect::from_min_size(Pos2::new(x, y), Vec2::new(seg_w, row_h)),
                0,
                col,
            );
        }
    }
}

/// The red, orange, yellow and cream stripe.
pub fn stripe(p: &Painter, rect: Rect) {
    let w = rect.width() / 4.0;
    for (i, c) in STRIPE.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(rect.left() + w * i as f32, rect.top()),
            Vec2::new(w + 0.5, rect.height()),
        );
        p.rect_filled(r, 0, *c);
    }
}

fn vgradient(p: &Painter, rect: Rect, stops: &[(f32, Color32)]) {
    let mut mesh = Mesh::default();
    for w in stops.windows(2) {
        let (t0, c0) = w[0];
        let (t1, c1) = w[1];
        let y0 = rect.top() + rect.height() * t0;
        let y1 = rect.top() + rect.height() * t1;
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(Pos2::new(rect.left(), y0), c0);
        mesh.colored_vertex(Pos2::new(rect.right(), y0), c0);
        mesh.colored_vertex(Pos2::new(rect.right(), y1), c1);
        mesh.colored_vertex(Pos2::new(rect.left(), y1), c1);
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    p.add(Shape::mesh(mesh));
}

/// An LED: lit red with a glow, or dark.
pub fn led(p: &Painter, c: Pos2, on: bool) {
    if on {
        p.circle_filled(c, 7.0, LED.gamma_multiply(0.12));
        p.circle_filled(c, 4.5, LED.gamma_multiply(0.3));
        p.circle_filled(c, 3.0, LED);
        p.circle_filled(
            c + Vec2::new(-0.8, -0.8),
            1.0,
            Color32::from_rgb(0xff, 0xc0, 0xa8),
        );
    } else {
        p.circle_filled(c, 3.0, LED_OFF);
        p.circle_stroke(c, 3.0, Stroke::new(0.6, Color32::from_black_alpha(160)));
    }
}

// ---------------------------------------------------------------- controls

/// A coloured rectangular hardware key with an optional LED above it and its
/// legend below.
pub fn key(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    colour: KeyColour,
    led_on: Option<bool>,
    label: &str,
    th: &Theme,
) -> Response {
    let hit = rect.expand2(Vec2::new(2.0, 10.0));
    let resp = ui.interact(hit, id, Sense::click());
    let p = ui.painter();
    let pressed = resp.is_pointer_button_down_on();
    let r = if pressed {
        rect.translate(Vec2::new(0.0, 1.0))
    } else {
        rect
    };
    p.rect_filled(
        rect.translate(Vec2::new(0.0, 3.0)),
        CornerRadius::same(2),
        Color32::from_black_alpha(150),
    );
    p.rect_filled(
        r.translate(Vec2::new(0.0, 1.5)),
        CornerRadius::same(2),
        colour.shadow,
    );
    p.rect_filled(r.expand(1.0), CornerRadius::same(2), Color32::BLACK);
    let face = if resp.hovered() {
        lerp_colour(colour.face, Color32::WHITE, 0.08)
    } else {
        colour.face
    };
    vgradient(
        p,
        r.shrink(0.5),
        &[
            (0.0, colour.hi),
            (0.18, face),
            (0.8, face),
            (1.0, colour.lo),
        ],
    );
    if resp.has_focus() {
        p.rect_stroke(
            r.expand(3.0),
            CornerRadius::same(3),
            Stroke::new(2.0, FOCUS),
            StrokeKind::Middle,
        );
    }
    if let Some(on) = led_on {
        led(p, Pos2::new(rect.center().x, rect.top() - 8.0), on);
    }
    if !label.is_empty() {
        legend(
            p,
            Pos2::new(rect.center().x, rect.bottom() + 5.0),
            Align2::CENTER_TOP,
            label,
            th,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Shared drag / scroll / keyboard handling for knobs and faders.
fn adjust(ui: &mut Ui, resp: &mut Response, id: Id, v: &mut f32, spec: &Spec, travel_px: f32) {
    let shift = ui.input(|i| i.modifiers.shift);
    let mut t = spec.norm(*v);
    let before = *v;
    let acc_id = id.with("acc");

    if resp.drag_started() || resp.clicked() {
        resp.request_focus();
    }
    if resp.drag_started() {
        ui.data_mut(|d| d.insert_temp(acc_id, t));
    }
    if resp.dragged() {
        let d = resp.drag_delta();
        let scale = if shift { travel_px * 8.0 } else { travel_px };
        let mut acc = ui.data(|d| d.get_temp::<f32>(acc_id)).unwrap_or(t);
        acc = (acc + (d.x - d.y) / scale).clamp(0.0, 1.0);
        ui.data_mut(|d| d.insert_temp(acc_id, acc));
        *v = spec.snap(spec.denorm(acc));
    }
    if resp.hovered() {
        let scroll = wheel(ui);
        if scroll != 0.0 {
            t += scroll.signum() * if shift { 0.002 } else { 0.01 };
            *v = spec.snap(spec.denorm(t));
        }
    }
    if resp.double_clicked() {
        *v = spec.default;
    }
    if resp.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let (up, down, pgup, pgdn, home, end) = ui.input(|i| {
            (
                i.key_pressed(Key::ArrowUp) || i.key_pressed(Key::ArrowRight),
                i.key_pressed(Key::ArrowDown) || i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::PageUp),
                i.key_pressed(Key::PageDown),
                i.key_pressed(Key::Home),
                i.key_pressed(Key::End),
            )
        });
        let dir = up as i32 - down as i32;
        if dir != 0 {
            if shift && spec.step > 0.0 {
                *v = spec.snap(*v + dir as f32 * spec.step);
            } else {
                // Step at least one grid unit so the value always moves.
                let mut nt = t + dir as f32 * 0.01;
                let mut nv = spec.snap(spec.denorm(nt));
                while nv == *v && (0.0..=1.0).contains(&nt) {
                    nt += dir as f32 * 0.002;
                    nv = spec.snap(spec.denorm(nt));
                }
                *v = nv;
            }
        }
        let page = pgup as i32 - pgdn as i32;
        if page != 0 {
            *v = spec.snap(spec.denorm(t + page as f32 * 0.1));
        }
        if home {
            *v = spec.min;
        }
        if end {
            *v = spec.max;
        }
    }
    if *v != before {
        resp.mark_changed();
    }
}

/// A knurled knob turning through 270°.
#[allow(clippy::too_many_arguments)]
pub fn knob(
    ui: &mut Ui,
    centre: Pos2,
    r: f32,
    id: Id,
    v: &mut f32,
    spec: &Spec,
    fam: Option<Color32>,
    th: &Theme,
) -> Response {
    let rect = Rect::from_center_size(centre, Vec2::splat(r * 2.0 + 10.0));
    let mut resp = ui.interact(rect, id, Sense::click_and_drag());
    adjust(ui, &mut resp, id, v, spec, 200.0);
    let p = ui.painter();
    let t = spec.norm(*v);
    let pointer_mode = th.knobs == KnobStyle::Pointer;
    let tick_col = match (pointer_mode, fam) {
        (true, Some(c)) => c,
        _ => th.tick,
    };

    // Tick scale
    for i in 0..=10 {
        let a = (-135.0 + 27.0 * i as f32).to_radians();
        let d = Vec2::new(a.sin(), -a.cos());
        let (r0, r1) = if i == 0 || i == 5 || i == 10 {
            (r + 3.0, r + 8.0)
        } else {
            (r + 4.0, r + 7.0)
        };
        p.line_segment(
            [centre + d * r0, centre + d * r1],
            Stroke::new(1.2, tick_col),
        );
    }
    // Shadow
    p.circle_filled(
        centre + Vec2::new(0.0, 4.0),
        r + 1.0,
        Color32::from_black_alpha(120),
    );
    // Knurled skirt
    let mut mesh = Mesh::default();
    const N: usize = 72;
    for i in 0..N {
        let a0 = i as f32 / N as f32 * 2.0 * PI;
        let a1 = (i + 1) as f32 / N as f32 * 2.0 * PI;
        let col = if i % 2 == 0 {
            Color32::from_rgb(6, 6, 6)
        } else {
            Color32::from_rgb(0x2c, 0x2c, 0x2c)
        };
        let b = mesh.vertices.len() as u32;
        mesh.colored_vertex(centre, col);
        mesh.colored_vertex(centre + Vec2::new(a0.cos(), a0.sin()) * r, col);
        mesh.colored_vertex(centre + Vec2::new(a1.cos(), a1.sin()) * r, col);
        mesh.add_triangle(b, b + 1, b + 2);
    }
    p.add(Shape::mesh(mesh));
    p.circle_stroke(centre, r, Stroke::new(1.0, Color32::BLACK));

    // Cap: a radial gradient lit from the upper left.
    let cap_r = r * 0.7;
    let (hi, mid, lo) = match (pointer_mode, fam) {
        (false, Some(c)) => (
            lerp_colour(c, Color32::WHITE, 0.3),
            c,
            lerp_colour(c, Color32::BLACK, 0.6),
        ),
        _ => (
            Color32::from_rgb(0x5a, 0x5a, 0x5a),
            Color32::from_rgb(0x2a, 0x2a, 0x2a),
            Color32::from_rgb(0x13, 0x13, 0x13),
        ),
    };
    p.circle_filled(centre, cap_r + 1.0, Color32::from_black_alpha(160));
    let light = centre + Vec2::new(-0.24, -0.44) * cap_r;
    const STEPS: usize = 14;
    for s in 0..STEPS {
        let f = s as f32 / (STEPS - 1) as f32; // 0 outer .. 1 inner
        let rad = cap_r * (1.0 - f * 0.92);
        let c = centre + (light - centre) * f;
        let col = if f < 0.5 {
            lerp_colour(lo, mid, f / 0.5)
        } else {
            lerp_colour(mid, hi, (f - 0.5) / 0.5)
        };
        p.circle_filled(c, rad, col);
    }
    p.circle_stroke(
        centre,
        cap_r,
        Stroke::new(0.8, Color32::from_white_alpha(25)),
    );

    // Pointer
    let a = (-135.0 + 270.0 * t).to_radians();
    let d = Vec2::new(a.sin(), -a.cos());
    let ptr_col = match (pointer_mode, fam) {
        (true, Some(c)) => c,
        _ => Color32::from_rgb(0xf4, 0xf0, 0xe6),
    };
    p.line_segment(
        [centre + d * r * 0.16, centre + d * r * 0.93],
        Stroke::new(3.5, Color32::from_black_alpha(140)),
    );
    p.line_segment(
        [centre + d * r * 0.16, centre + d * r * 0.92],
        Stroke::new(2.6, ptr_col),
    );

    if resp.has_focus() {
        p.circle_stroke(centre, r + 3.0, Stroke::new(2.0, FOCUS));
    }
    resp.on_hover_cursor(egui::CursorIcon::ResizeVertical)
}

/// A Jupiter-style fader: black slot, tick marks, black cap with a line.
pub fn fader(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    v: &mut f32,
    spec: &Spec,
    line: Option<Color32>,
    th: &Theme,
) -> Response {
    let mut resp = ui.interact(rect, id, Sense::click_and_drag());
    let top = rect.top() + 12.0;
    let bottom = rect.bottom() - 12.0;
    let travel = bottom - top;
    // Clicking the track (not the cap) jumps there.
    if resp.clicked()
        || (resp.drag_started()
            && !cap_rect(rect, spec.norm(*v))
                .contains(resp.interact_pointer_pos().unwrap_or_default()))
    {
        if let Some(pos) = resp.interact_pointer_pos() {
            let nt = ((bottom - pos.y) / travel).clamp(0.0, 1.0);
            let nv = spec.snap(spec.denorm(nt));
            if nv != *v {
                *v = nv;
                resp.mark_changed();
            }
        }
    }
    adjust(ui, &mut resp, id, v, spec, travel);
    let p = ui.painter();
    let cx = rect.center().x;
    // Ticks
    let default_t = spec.norm(spec.default);
    for i in 0..=12 {
        let tt = i as f32 / 12.0;
        let y = bottom - tt * travel;
        let major = i == 0 || i == 12 || (tt - default_t).abs() < 0.01;
        let w = if major { 2.0 } else { 1.0 };
        p.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.left() + 7.0, y)],
            Stroke::new(w, th.tick),
        );
        p.line_segment(
            [Pos2::new(rect.right() - 7.0, y), Pos2::new(rect.right(), y)],
            Stroke::new(w, th.tick),
        );
    }
    // Slot
    let slot = Rect::from_min_max(
        Pos2::new(cx - 3.0, top - 2.0),
        Pos2::new(cx + 3.0, bottom + 2.0),
    );
    p.rect_filled(
        slot.translate(Vec2::new(0.0, 1.0)),
        CornerRadius::same(3),
        Color32::from_white_alpha(22),
    );
    p.rect_filled(slot, CornerRadius::same(3), Color32::BLACK);
    // Cap
    let cap = cap_rect(rect, spec.norm(*v));
    p.rect_filled(
        cap.translate(Vec2::new(0.0, 4.0)),
        CornerRadius::same(2),
        Color32::from_black_alpha(140),
    );
    p.rect_filled(cap.expand(1.0), CornerRadius::same(2), Color32::BLACK);
    vgradient(
        p,
        cap,
        &[
            (0.0, Color32::from_rgb(0x50, 0x50, 0x50)),
            (0.28, Color32::from_rgb(0x20, 0x20, 0x20)),
            (0.5, Color32::from_rgb(0x0b, 0x0b, 0x0b)),
            (0.72, Color32::from_rgb(0x1d, 0x1d, 0x1d)),
            (1.0, Color32::from_rgb(0x3c, 0x3c, 0x3c)),
        ],
    );
    let line_col = line.unwrap_or(Color32::from_rgb(0xf4, 0xf0, 0xe6));
    p.rect_filled(
        Rect::from_center_size(cap.center(), Vec2::new(cap.width() - 8.0, 2.0)),
        CornerRadius::same(1),
        line_col,
    );
    if resp.has_focus() {
        p.rect_stroke(
            cap.expand(2.0),
            CornerRadius::same(3),
            Stroke::new(2.0, FOCUS),
            StrokeKind::Middle,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::ResizeVertical)
}

fn cap_rect(rect: Rect, t: f32) -> Rect {
    let top = rect.top() + 12.0;
    let bottom = rect.bottom() - 12.0;
    let y = bottom - t * (bottom - top);
    Rect::from_center_size(
        Pos2::new(rect.center().x, y),
        Vec2::new(rect.width() - 4.0, 24.0),
    )
}

/// A drop-down list drawn as a red display. Returns the newly picked index.
pub fn lcd_combo(
    ui: &mut Ui,
    rect: Rect,
    id: &str,
    items: &[String],
    selected: usize,
) -> Option<usize> {
    let mut picked = None;
    let current = items.get(selected).cloned().unwrap_or_else(|| "—".into());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        let style = ui.style_mut();
        for w in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            w.bg_fill = LCD_BG;
            w.weak_bg_fill = LCD_BG;
            w.fg_stroke = Stroke::new(1.0, LCD_INK);
            w.bg_stroke = Stroke::new(1.0, Color32::BLACK);
            w.corner_radius = CornerRadius::same(2);
            w.expansion = 0.0;
        }
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, LCD_DIM);
        style.override_font_id = Some(font(F_MONO, 13.0));
        style.spacing.button_padding = Vec2::new(8.0, 4.0);
        egui::ComboBox::from_id_salt(id)
            .width(rect.width())
            .truncate()
            .selected_text(egui::RichText::new(current.to_uppercase()).color(LCD_INK))
            .show_ui(ui, |ui| {
                for (i, item) in items.iter().enumerate() {
                    if ui.selectable_label(i == selected, item).clicked() && i != selected {
                        picked = Some(i);
                    }
                }
            });
    });
    picked
}

/// Mouse-wheel movement this frame (positive is up / away).
pub fn wheel(ui: &Ui) -> f32 {
    ui.input(|i| {
        i.events
            .iter()
            .map(|e| match e {
                egui::Event::MouseWheel { delta, .. } => delta.y,
                _ => 0.0,
            })
            .sum()
    })
}
