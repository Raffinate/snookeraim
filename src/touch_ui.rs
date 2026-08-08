use raylib::prelude::*;

// On-screen touch controls + help popup: sized for fingers, not just
// mouse pointers, and laid out from the current screen size each frame so
// they hold up across window resizes and phone aspect ratios. These are
// the *base* (desktop-scale) sizes; `touch_ui` scales them down together
// -- down to `UI_MIN_SCALE` -- so the whole control set still fits (and
// doesn't overlap itself) on small phone screens.
const BTN_BASE: i32 = 52; // button edge length, px, at scale 1.0
const BTN_GAP_BASE: i32 = 8;
const BTN_MARGIN_BASE: i32 = 14;
const BTN_FONT_BASE: i32 = 16;
const BTN_FONT_MIN: i32 = 10;
const UI_MIN_SCALE: f32 = 0.55;
// Dark, fairly opaque fill (not the light near-transparent white this used
// to be) so the white text stays readable against any backdrop -- the
// white gallery room made a white-tinted, low-alpha button nearly
// invisible, with almost no contrast for its own white text.
const BTN_FILL: Color = Color::new(20, 20, 20, 150);
const BTN_FILL_HOVER: Color = Color::new(60, 60, 60, 170);
const BTN_FILL_ACTIVE: Color = Color::new(255, 200, 30, 210);
const BTN_BORDER: Color = Color::new(0, 0, 0, 180);
const BTN_TEXT: Color = Color::WHITE;
pub const HELP_BG: Color = Color::new(0, 0, 0, 190);

fn point_in_rect(px: f32, py: f32, x: i32, y: i32, w: i32, h: i32) -> bool {
    px >= x as f32 && px <= (x + w) as f32 && py >= y as f32 && py <= (y + h) as f32
}

/// A single on-screen touch/click button. Positions are recomputed from
/// the current screen size every frame (see `touch_ui`) rather than
/// stored, so the layout holds up across window resizes.
pub struct Btn {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &'static str,
    font: i32,
}

impl Btn {
    fn new(x: i32, y: i32, w: i32, h: i32, label: &'static str, font: i32) -> Self {
        Btn { x, y, w, h, label, font }
    }

    pub fn hit(&self, mouse: Vector2) -> bool {
        point_in_rect(mouse.x, mouse.y, self.x, self.y, self.w, self.h)
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle, mouse: Vector2, active: bool) {
        let hovered = self.hit(mouse);
        let fill = if active {
            BTN_FILL_ACTIVE
        } else if hovered {
            BTN_FILL_HOVER
        } else {
            BTN_FILL
        };
        d.draw_rectangle(self.x, self.y, self.w, self.h, fill);
        d.draw_rectangle_lines(self.x, self.y, self.w, self.h, BTN_BORDER);
        let text_w = d.measure_text(self.label, self.font);
        d.draw_text(
            self.label,
            self.x + (self.w - text_w) / 2,
            self.y + (self.h - self.font) / 2,
            self.font,
            BTN_TEXT,
        );
    }
}

/// All on-screen controls: a camera cluster (pan/rotate/zoom) bottom-left,
/// action buttons (mirroring the keyboard hotkeys) bottom-right, and a
/// help toggle top-right. Rebuilt fresh from the current screen size every
/// frame -- cheap (a couple dozen integer adds), and the only way the
/// layout stays correct as the window/canvas is resized.
pub struct TouchUi {
    pub pan_up: Btn,
    pub pan_down: Btn,
    pub pan_left: Btn,
    pub pan_right: Btn,
    pub rot_left: Btn,
    pub rot_right: Btn,
    pub zoom_in: Btn,
    pub zoom_out: Btn,
    pub reset: Btn,
    pub test: Btn,
    // `None` while collapsed via `expand_toggle` -- not drawn, not
    // hit-tested, no space reserved for them.
    pub ghost: Option<Btn>,
    pub aim: Option<Btn>,
    pub close_stance: Btn,
    pub stand_stance: Btn,
    pub pot_line: Btn,
    pub expand_toggle: Btn,
    pub view: Btn,
    pub center: Btn,
    pub help: Btn,
}

pub fn opt_hit(btn: &Option<Btn>, mouse: Vector2) -> bool {
    btn.as_ref().is_some_and(|b| b.hit(mouse))
}

/// The ESC pause menu: CONTINUE always, QUIT only on native builds. A web
/// build runs inside a browser tab that the page itself has no way to
/// close, so rather than show a QUIT button that can't actually do
/// anything, it just doesn't exist there -- same `Option<Btn>` pattern as
/// the collapsible GHOST/AIM buttons above.
pub struct MenuUi {
    pub continue_btn: Btn,
    pub quit_btn: Option<Btn>,
    title_y: i32,
}

impl MenuUi {
    pub fn draw(&self, d: &mut RaylibDrawHandle, mouse: Vector2, screen_w: i32, screen_h: i32) {
        d.draw_rectangle(0, 0, screen_w, screen_h, HELP_BG);
        let title = "PAUSED";
        let title_size = 32;
        let title_w = d.measure_text(title, title_size);
        d.draw_text(title, screen_w / 2 - title_w / 2, self.title_y, title_size, Color::RAYWHITE);
        self.continue_btn.draw(d, mouse, false);
        if let Some(b) = &self.quit_btn {
            b.draw(d, mouse, false);
        }
    }
}

pub fn menu_ui(screen_w: i32, screen_h: i32) -> MenuUi {
    let btn_w = 260;
    let btn_h = 64;
    let gap = 20;
    let font = 22;

    #[cfg(target_os = "emscripten")]
    let has_quit = false;
    #[cfg(not(target_os = "emscripten"))]
    let has_quit = true;

    let rows = if has_quit { 2 } else { 1 };
    let stack_h = rows * btn_h + (rows - 1) * gap;
    let x = screen_w / 2 - btn_w / 2;
    let start_y = screen_h / 2 - stack_h / 2;

    let continue_btn = Btn::new(x, start_y, btn_w, btn_h, "CONTINUE", font);
    let quit_btn = if has_quit {
        Some(Btn::new(x, start_y + btn_h + gap, btn_w, btn_h, "QUIT", font))
    } else {
        None
    };

    MenuUi { continue_btn, quit_btn, title_y: start_y - 60 }
}

impl TouchUi {
    pub fn hit_any(&self, mouse: Vector2) -> bool {
        [
            &self.pan_up,
            &self.pan_down,
            &self.pan_left,
            &self.pan_right,
            &self.rot_left,
            &self.rot_right,
            &self.zoom_in,
            &self.zoom_out,
            &self.reset,
            &self.test,
            &self.expand_toggle,
            &self.view,
            &self.center,
            &self.help,
            &self.close_stance,
            &self.stand_stance,
            &self.pot_line,
        ]
        .iter()
        .any(|b| b.hit(mouse))
            || opt_hit(&self.ghost, mouse)
            || opt_hit(&self.aim, mouse)
    }
}

pub fn touch_ui(
    screen_w: i32,
    screen_h: i32,
    reset_label: &'static str,
    ghost_aim_visible: bool,
) -> TouchUi {
    // Scale the whole control set down together so it always fits the
    // current screen without overlapping itself, with a floor so buttons
    // never shrink below tappable. The bottom-left camera cluster and the
    // bottom-right HIT square are the two widest bottom-anchored things
    // (each `6*BTN+5*GAP` / `3*BTN+2*GAP` wide); the top-right column and
    // that same HIT square are the two tallest. Solving for the scale that
    // keeps each pair's combined footprint within the screen (with a
    // little slack for margins) gives a hard upper bound; below that we
    // just use the screen as-is.
    let bl_w_base = 6 * BTN_BASE + 5 * BTN_GAP_BASE;
    let hit_w_base = 3 * BTN_BASE + 2 * BTN_GAP_BASE;
    let top_h_base = 3 * BTN_BASE + 2 * BTN_GAP_BASE;
    // Bottom-left is now 4 rows tall: the preset row, LOOK, and the two
    // D-pad rows. height_scale needs the taller of the two corners' column
    // heights, not just the top-right one, or a tall bottom-left cluster
    // could overflow a short screen unnoticed.
    let bl_h_base = 4 * BTN_BASE + 3 * BTN_GAP_BASE;
    let width_scale = (screen_w - 3 * BTN_MARGIN_BASE) as f32 / (bl_w_base + hit_w_base) as f32;
    let height_scale =
        (screen_h - 3 * BTN_MARGIN_BASE) as f32 / (top_h_base.max(bl_h_base) + hit_w_base) as f32;
    let scale = width_scale.min(height_scale).min(1.0).max(UI_MIN_SCALE);

    let btn = (BTN_BASE as f32 * scale).round() as i32;
    let gap = ((BTN_GAP_BASE as f32 * scale).round() as i32).max(2);
    let margin = (BTN_MARGIN_BASE as f32 * scale).round() as i32;
    let font = ((BTN_FONT_BASE as f32 * scale).round() as i32).max(BTN_FONT_MIN);
    let step = btn + gap;

    // Bottom-left: a 4x2 grid (Q ^ E + / < v > -), a square 2x2 CENTER
    // button to its right, and a LOOK header bar spanning both on top.
    let grid_x = margin;
    let row_bottom_y = screen_h - margin - btn;
    let row_top_y = row_bottom_y - step;
    let look_y = row_top_y - step;
    let col_x = |col: i32| grid_x + col * step;

    let rot_left = Btn::new(col_x(0), row_top_y, btn, btn, "Q", font);
    let pan_up = Btn::new(col_x(1), row_top_y, btn, btn, "^", font);
    let rot_right = Btn::new(col_x(2), row_top_y, btn, btn, "E", font);
    let zoom_in = Btn::new(col_x(3), row_top_y, btn, btn, "+", font);

    let pan_left = Btn::new(col_x(0), row_bottom_y, btn, btn, "<", font);
    let pan_down = Btn::new(col_x(1), row_bottom_y, btn, btn, "v", font);
    let pan_right = Btn::new(col_x(2), row_bottom_y, btn, btn, ">", font);
    let zoom_out = Btn::new(col_x(3), row_bottom_y, btn, btn, "-", font);

    let center_w = 2 * btn + gap;
    let center_x = col_x(3) + step;
    let center = Btn::new(center_x, row_top_y, center_w, center_w, "CENTER", font);

    let look_w = (center_x + center_w) - grid_x;
    let view = Btn::new(grid_x, look_y, look_w, btn, "LOOK", font);

    // Camera presets, one row directly above LOOK, same combined width --
    // grouped with the rest of the camera cluster since that's what they
    // are, rather than tucked away behind the top-right expand toggle.
    let preset_y = look_y - step;
    let preset_w = (look_w - 2 * gap) / 3;
    let close_stance = Btn::new(grid_x, preset_y, preset_w, btn, "CLOSE", font);
    let stand_stance = Btn::new(grid_x + preset_w + gap, preset_y, preset_w, btn, "STAND", font);
    let pot_line = Btn::new(grid_x + 2 * (preset_w + gap), preset_y, preset_w, btn, "LINE", font);

    // Bottom-right: HIT stands alone, mirroring the Space hotkey -- a big
    // 3x3 square, since it's the main "do the thing" action.
    let hit_w = 3 * btn + 2 * gap;
    let test = Btn::new(
        screen_w - margin - hit_w,
        screen_h - margin - hit_w,
        hit_w,
        hit_w,
        "HIT",
        font,
    );

    // Top-right: help, with an expand/collapse toggle directly below it.
    // Reset sits to help's left, two squares wide so its "CLEAR"/"NEXT"
    // label has room to breathe, and its label reflects what pressing it
    // will actually do right now. Ghost/Aim stack below reset, sharing its
    // width, but only exist (and only take up space) while expanded.
    let help = Btn::new(screen_w - margin - btn, margin, btn, btn, "?", font);
    let expand_toggle = Btn::new(
        help.x,
        margin + step,
        btn,
        btn,
        if ghost_aim_visible { ">>" } else { "<<" },
        font,
    );
    let reset_w = 2 * btn + gap;
    let reset_x = help.x - gap - reset_w;
    let reset = Btn::new(reset_x, margin, reset_w, btn, reset_label, font);
    let (ghost, aim) = if ghost_aim_visible {
        (
            Some(Btn::new(reset_x, margin + step, reset_w, btn, "GHOST", font)),
            Some(Btn::new(reset_x, margin + 2 * step, reset_w, btn, "AIM", font)),
        )
    } else {
        (None, None)
    };

    TouchUi {
        pan_up,
        pan_down,
        pan_left,
        pan_right,
        rot_left,
        rot_right,
        zoom_in,
        zoom_out,
        reset,
        test,
        ghost,
        aim,
        close_stance,
        stand_stance,
        pot_line,
        expand_toggle,
        view,
        center,
        help,
    }
}
