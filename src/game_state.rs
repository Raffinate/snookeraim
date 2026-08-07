use raylib::prelude::*;

use crate::assets::Assets;
use crate::camera::{
    aiming_camera, apply_aim_stance, cursor_table_point, pot_line_camera, rotate_sensitivity,
    screen_ray_table_point, zoom_toward, CAMERA_CLOSE_BACK_DISTANCE, CAMERA_CLOSE_ELEVATION_DEG,
    CAMERA_STANCE_BACK_DISTANCE, CAMERA_STANCE_ELEVATION_DEG, CAMERA_STANCE_LATERAL_OFFSET,
    KEY_ROTATE_SPEED_DEG, PAN_SPEED, PINCH_ZOOM_SENSITIVITY, ZOOM_BUTTON_SPEED,
};
use crate::cue::{draw_cue, draw_cue_model};
use crate::cushion_segments::{CUSHION_BOUNDARY, SHORT_RAIL_BOUNDARY};
use crate::shot::{
    best_pocket, cue_raycast, draw_gate, draw_object_ball_aim_line, draw_path_stripe,
    random_shot_setup, test_shot, GateState, ShotTest, GATE_MISS_COLOR, GATE_NEUTRAL_COLOR,
    GATE_SUCCESS_COLOR, GHOST_BALL_COLOR, GHOST_RED_BALL_COLOR, PATH_RED_COLOR, PATH_WHITE_COLOR,
};
use crate::table::{
    draw_ball_collision_ring, draw_light_fixture, draw_table, Pocket, CUE_BALL_COLOR,
    CUE_BALL_MESH_INDEX, CUE_BALL_MODEL_CENTER, GALLERY_MODEL_OFFSET_X, GALLERY_MODEL_OFFSET_Y,
    GALLERY_MODEL_OFFSET_Z, OBJECT_BALL_COLOR, RED_BALL_MESH_INDEX, RED_BALL_MODEL_CENTER,
    TABLE_MODEL_OFFSET_X, TABLE_MODEL_OFFSET_Y, TABLE_MODEL_OFFSET_Z, USE_GALLERY_MODEL,
    USE_MODEL_PROPS, USE_TABLE_MODEL,
};
use crate::touch_ui::{opt_hit, TouchUi, HELP_BG};

/// `draw_mesh` consumes its material by value; this hands it a throwaway
/// non-owning copy so the real material survives to the next frame.
fn weak_copy(material: &WeakMaterial) -> WeakMaterial {
    unsafe { WeakMaterial::from_raw(*material.as_ref()) }
}

/// All per-frame mutable game state: camera, ball positions, the current
/// target pocket, a frozen shot-test result (if any), and the various
/// toggles/view-mode bookkeeping the input-handling loop mutates. Loaded
/// assets (shaders/models/meshes) live separately in `Assets`, since they're
/// set up once and never change shape after that.
pub struct GameState {
    pub(crate) camera: Camera3D,
    pub(crate) cue_ball_pos: Vector3,
    pub(crate) object_ball_pos: Vector3,
    pub(crate) target_pocket: (usize, (f32, f32)),
    pub(crate) shot_test: Option<ShotTest>,
    pub(crate) view_mode: bool,
    pub(crate) saved_camera: Option<Camera3D>,
    pub(crate) last_view_camera: Option<Camera3D>,
    pub(crate) frozen_aim_camera: Camera3D,
    pub(crate) show_ghost_ball: bool,
    pub(crate) show_aim_line: bool,
    pub(crate) help_visible: bool,
    pub(crate) ghost_aim_visible: bool,
    pub(crate) show_collision_debug: bool,
    pub(crate) pinch_prev_dist: Option<f32>,
}

impl GameState {
    pub fn new(pockets: &[Pocket]) -> GameState {
        let (cue_ball_pos, object_ball_pos) = random_shot_setup(pockets);
        let camera = aiming_camera(cue_ball_pos, object_ball_pos);
        let (pocket_idx, gate_dir, _) = best_pocket(pockets, cue_ball_pos, object_ball_pos);
        GameState {
            camera,
            cue_ball_pos,
            object_ball_pos,
            target_pocket: (pocket_idx, gate_dir),
            shot_test: None,
            show_ghost_ball: true,
            show_aim_line: false,
            view_mode: false,
            saved_camera: None,
            last_view_camera: None,
            frozen_aim_camera: camera,
            help_visible: false,
            ghost_aim_visible: true,
            show_collision_debug: false,
            pinch_prev_dist: None,
        }
    }

    /// While in view mode, the cue's aim stays frozen at whatever it was
    /// when view mode was entered, even as the camera keeps moving.
    pub fn aim_camera(&self) -> Camera3D {
        if self.view_mode { self.frozen_aim_camera } else { self.camera }
    }

    /// Shift+`\`` toggles the collision-debug overlay; `?`/the help button
    /// toggles the help popup, and any other tap while it's open dismisses
    /// it (the help button itself is excluded by the branch above).
    pub fn handle_global_toggles(&mut self, rl: &RaylibHandle, ui: &TouchUi, mouse: Vector2, tap: bool) {
        if rl.is_key_pressed(KeyboardKey::KEY_GRAVE)
            && (rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                || rl.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT))
        {
            self.show_collision_debug = !self.show_collision_debug;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_SLASH) || (tap && ui.help.hit(mouse)) {
            self.help_visible = !self.help_visible;
        } else if self.help_visible && tap {
            self.help_visible = false;
        }
    }

    /// Space / HIT: traces both balls' paths for the current aim and freezes
    /// the result until the next test or reset.
    pub fn maybe_test_shot(
        &mut self,
        rl: &RaylibHandle,
        ui: &TouchUi,
        mouse: Vector2,
        tap: bool,
        shot_dir: Option<(f32, f32)>,
        pockets: &[Pocket],
    ) {
        if let (true, Some(dir)) = (
            rl.is_key_pressed(KeyboardKey::KEY_SPACE) || (tap && ui.test.hit(mouse)),
            shot_dir,
        ) {
            let (pocket_idx, gate_dir) = self.target_pocket;
            self.shot_test = Some(test_shot(
                dir,
                self.cue_ball_pos,
                self.object_ball_pos,
                pockets[pocket_idx].position,
                gate_dir,
            ));
        }
    }

    /// Pan (WASD/arrows/D-pad), rotate (Q/E/D-pad), drag-orbit, and
    /// wheel/pinch/on-screen-button zoom.
    pub fn handle_camera_movement(
        &mut self,
        rl: &RaylibHandle,
        ui: &TouchUi,
        mouse: Vector2,
        held: bool,
        tap: bool,
        over_ui: bool,
    ) {
        let pan_dist = PAN_SPEED * rl.get_frame_time();
        if rl.is_key_down(KeyboardKey::KEY_W)
            || rl.is_key_down(KeyboardKey::KEY_UP)
            || (held && ui.pan_up.hit(mouse))
        {
            self.camera.move_forward(pan_dist, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_S)
            || rl.is_key_down(KeyboardKey::KEY_DOWN)
            || (held && ui.pan_down.hit(mouse))
        {
            self.camera.move_forward(-pan_dist, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_A)
            || rl.is_key_down(KeyboardKey::KEY_LEFT)
            || (held && ui.pan_left.hit(mouse))
        {
            self.camera.move_right(-pan_dist, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_D)
            || rl.is_key_down(KeyboardKey::KEY_RIGHT)
            || (held && ui.pan_right.hit(mouse))
        {
            self.camera.move_right(pan_dist, true);
        }

        let key_rotate = KEY_ROTATE_SPEED_DEG.to_radians() * rl.get_frame_time();
        if rl.is_key_down(KeyboardKey::KEY_Q) || (held && ui.rot_left.hit(mouse)) {
            self.camera.yaw(key_rotate, true);
        }
        if rl.is_key_down(KeyboardKey::KEY_E) || (held && ui.rot_right.hit(mouse)) {
            self.camera.yaw(-key_rotate, true);
        }

        let touch_count = rl.get_touch_point_count();

        if held && !over_ui && !tap && touch_count < 2 {
            // Skip the exact press frame: on touch devices there's no
            // cursor easing into position beforehand, so the browser's
            // synthesized mouse position jumps straight from wherever it
            // last was to the tap point on this same frame. Reading that
            // as a drag delta would snap the camera toward the tap
            // instead of orbiting from it. Also skip while a second
            // finger is down -- that's a pinch, not a one-finger orbit,
            // and the browser's synthesized mouse position (tracking
            // just the first finger) would otherwise fight the pinch.
            let delta = rl.get_mouse_delta();
            let sensitivity = rotate_sensitivity(rl, self.camera, self.object_ball_pos);
            self.camera.yaw(-delta.x * sensitivity, true);
            self.camera.pitch(-delta.y * sensitivity, true, true, false);
        }
        let wheel = rl.get_mouse_wheel_move();
        if wheel != 0.0 && !over_ui {
            // Zoom toward whatever world point is under the cursor so that
            // point stays fixed on screen as we zoom, instead of always
            // zooming to the orbit target.
            if let Some(hit) = cursor_table_point(rl, self.camera) {
                let factor = (wheel * 0.15).clamp(-0.9, 0.9);
                zoom_toward(&mut self.camera, hit, factor);
            }
        }

        // Pinch-zoom: two fingers down, zoom by how their spread
        // changes frame to frame (spreading apart = zoom in), toward
        // the point under their midpoint.
        if touch_count >= 2 {
            let t0 = rl.get_touch_position(0);
            let t1 = rl.get_touch_position(1);
            let dist = t0.distance(t1);
            let midpoint = t0.lerp(t1, 0.5);
            if let Some(prev) = self.pinch_prev_dist {
                let hit = screen_ray_table_point(rl, self.camera, midpoint)
                    .or_else(|| cursor_table_point(rl, self.camera));
                if let Some(hit) = hit {
                    let factor = ((dist - prev) * PINCH_ZOOM_SENSITIVITY).clamp(-0.9, 0.9);
                    zoom_toward(&mut self.camera, hit, factor);
                }
            }
            self.pinch_prev_dist = Some(dist);
        } else {
            self.pinch_prev_dist = None;
        }

        // On-screen zoom buttons: there's no cursor-hit point to zoom
        // toward (the cursor is on the button, not over the table), so
        // these just zoom straight along the existing camera->target axis.
        let mut button_zoom = 0.0;
        if held && ui.zoom_in.hit(mouse) {
            button_zoom -= ZOOM_BUTTON_SPEED * rl.get_frame_time();
        }
        if held && ui.zoom_out.hit(mouse) {
            button_zoom += ZOOM_BUTTON_SPEED * rl.get_frame_time();
        }
        if button_zoom != 0.0 {
            let dist = (self.camera.position.distance(self.camera.target) + button_zoom)
                .clamp(0.15, 6.0);
            let dir = (self.camera.position - self.camera.target).normalize();
            self.camera.position = self.camera.target + dir.scale(dist);
        }
    }

    /// The expand/collapse toggle, reset/next-layout, re-center, view-mode
    /// toggle, and ghost-ball/aim-line toggles.
    pub fn handle_reset_and_toggles(
        &mut self,
        rl: &RaylibHandle,
        ui: &TouchUi,
        mouse: Vector2,
        tap: bool,
        pockets: &[Pocket],
    ) {
        if tap && ui.expand_toggle.hit(mouse) {
            self.ghost_aim_visible = !self.ghost_aim_visible;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_R) || (tap && ui.reset.hit(mouse)) {
            if self.shot_test.is_some() {
                // A tested shot is on screen (paths/gate result) — first R
                // just clears that, rather than jumping straight to a new
                // layout.
                self.shot_test = None;
            } else {
                (self.cue_ball_pos, self.object_ball_pos) = random_shot_setup(pockets);
                let (pocket_idx, gate_dir, _) =
                    best_pocket(pockets, self.cue_ball_pos, self.object_ball_pos);
                self.target_pocket = (pocket_idx, gate_dir);
                self.camera = aiming_camera(self.cue_ball_pos, self.object_ball_pos);
                self.view_mode = false;
                self.saved_camera = None;
                self.last_view_camera = None;
            }
        }

        if rl.is_key_pressed(KeyboardKey::KEY_C) || (tap && ui.center.hit(mouse)) {
            // Re-center the orbit pivot on the cue ball without touching
            // zoom or viewing angle: shift both position and target by the
            // same offset, so their relative vector is unchanged.
            let offset = self.cue_ball_pos - self.camera.target;
            self.camera.target = self.cue_ball_pos;
            self.camera.position = self.camera.position + offset;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_V) || (tap && ui.view.hit(mouse)) {
            if self.view_mode {
                // Exiting: remember exactly where we were so re-entering
                // resumes here, then snap back to the pre-view-mode camera.
                self.last_view_camera = Some(self.camera);
                if let Some(saved) = self.saved_camera.take() {
                    self.camera = saved;
                }
            } else {
                // The frozen aim must come from the normal-mode camera (the
                // one the player actually aims with) — captured here,
                // before any swap to a resumed free-roam view below.
                // Otherwise, re-entering after having zoomed/orbited around
                // while inspecting would redefine the "frozen" aim from
                // that free-roam drift instead of the real aim.
                if self.last_view_camera.is_none() {
                    // First time entering this session: re-center the orbit
                    // pivot on the cue ball (instead of whatever the target
                    // happened to be) so rotating while inspecting orbits
                    // around the ball, not some unrelated leftover point.
                    let offset = self.cue_ball_pos - self.camera.target;
                    self.camera.target = self.cue_ball_pos;
                    self.camera.position = self.camera.position + offset;
                }
                self.frozen_aim_camera = self.camera;
                self.saved_camera = Some(self.camera);

                if let Some(resume) = self.last_view_camera {
                    self.camera = resume;
                }
            }
            self.view_mode = !self.view_mode;
        }

        if rl.is_key_pressed(KeyboardKey::KEY_G) || (tap && opt_hit(&ui.ghost, mouse)) {
            self.show_ghost_ball = !self.show_ghost_ball;
        }
        if rl.is_key_pressed(KeyboardKey::KEY_H) || (tap && opt_hit(&ui.aim, mouse)) {
            self.show_aim_line = !self.show_aim_line;
        }
    }

    /// Camera presets 1/2/3. 1 and 2 are aiming-mode stances -- if view mode
    /// is active they first exit it exactly like pressing V would (so
    /// re-entering view mode later still resumes the free-roam camera the
    /// player was inspecting from), then reposition the aim camera without
    /// touching its bearing. 3 is a view-mode preset -- it enters view mode
    /// like V would (so the current aim gets frozen for the cue), then
    /// overrides the resulting camera with the fixed potting-line vantage.
    pub fn handle_camera_presets(
        &mut self,
        rl: &RaylibHandle,
        ui: &TouchUi,
        mouse: Vector2,
        tap: bool,
        pockets: &[Pocket],
    ) {
        let close_pressed =
            rl.is_key_pressed(KeyboardKey::KEY_ONE) || (tap && ui.close_stance.hit(mouse));
        let stand_pressed =
            rl.is_key_pressed(KeyboardKey::KEY_TWO) || (tap && ui.stand_stance.hit(mouse));
        let pot_line_pressed =
            rl.is_key_pressed(KeyboardKey::KEY_THREE) || (tap && ui.pot_line.hit(mouse));

        if close_pressed || stand_pressed {
            if self.view_mode {
                self.last_view_camera = Some(self.camera);
                if let Some(saved) = self.saved_camera.take() {
                    self.camera = saved;
                }
                self.view_mode = false;
            }
            if close_pressed {
                apply_aim_stance(
                    &mut self.camera,
                    self.cue_ball_pos,
                    self.object_ball_pos,
                    CAMERA_CLOSE_BACK_DISTANCE,
                    CAMERA_CLOSE_ELEVATION_DEG,
                    0.0,
                );
            } else {
                apply_aim_stance(
                    &mut self.camera,
                    self.cue_ball_pos,
                    self.object_ball_pos,
                    CAMERA_STANCE_BACK_DISTANCE,
                    CAMERA_STANCE_ELEVATION_DEG,
                    CAMERA_STANCE_LATERAL_OFFSET,
                );
            }
        }

        if pot_line_pressed {
            if !self.view_mode {
                if self.last_view_camera.is_none() {
                    let offset = self.cue_ball_pos - self.camera.target;
                    self.camera.target = self.cue_ball_pos;
                    self.camera.position = self.camera.position + offset;
                }
                self.frozen_aim_camera = self.camera;
                self.saved_camera = Some(self.camera);
                self.view_mode = true;
            }
            self.camera = pot_line_camera(self.object_ball_pos, pockets[self.target_pocket.0].position);
        }
    }

    /// The full 3D scene: table, lights, collision-debug overlay, both
    /// balls, cue, ghost ball + aim line, gate, and any frozen shot-test
    /// paths.
    pub fn draw_scene(&self, d3: &mut (impl RaylibDraw3D + RaylibRlgl), assets: &mut Assets, shot_dir: Option<(f32, f32)>) {
        // A bit see-through in collision-debug mode so the boundary lines
        // and ball collision circles (both drawn on top) aren't hidden
        // behind the real geometry they're describing.
        let debug_tint = if self.show_collision_debug {
            Color::new(255, 255, 255, 130)
        } else {
            Color::WHITE
        };

        if USE_GALLERY_MODEL {
            // The source model's floor/ceiling/wall meshes weren't authored
            // double-sided and (unlike the table/cue/ball props, which were
            // always meant to be viewed from outside) this room is viewed
            // from *inside* it, where those single-sided normals face away
            // from the camera and get back-face culled -- disable culling
            // for just this draw so both faces of every triangle render.
            let offset = Vector3::new(GALLERY_MODEL_OFFSET_X, GALLERY_MODEL_OFFSET_Y, GALLERY_MODEL_OFFSET_Z);
            d3.rl_disable_backface_culling();
            d3.draw_model(&assets.gallery_model, offset, 1.0, Color::WHITE);
            d3.rl_enable_backface_culling();
        }

        if USE_TABLE_MODEL {
            let offset = Vector3::new(TABLE_MODEL_OFFSET_X, TABLE_MODEL_OFFSET_Y, TABLE_MODEL_OFFSET_Z);
            d3.draw_model(&assets.table_model, offset, 1.0, debug_tint);
        } else {
            draw_table(d3, &assets.pockets);
        }
        draw_light_fixture(d3, &assets.light_panels);

        if self.show_collision_debug {
            // Both tables only cover their own non-negative half (each
            // rail is symmetric about its own center), so mirror both
            // X and Z for each.
            for w in CUSHION_BOUNDARY.windows(2) {
                let ([z0, x0], [z1, x1]) = (w[0], w[1]);
                for &sx in &[1.0, -1.0] {
                    for &sz in &[1.0, -1.0] {
                        d3.draw_line3D(
                            Vector3::new(sx * x0, 0.02, sz * z0),
                            Vector3::new(sx * x1, 0.02, sz * z1),
                            Color::MAGENTA,
                        );
                    }
                }
            }
            for w in SHORT_RAIL_BOUNDARY.windows(2) {
                let ([x0, z0], [x1, z1]) = (w[0], w[1]);
                for &sx in &[1.0, -1.0] {
                    for &sz in &[1.0, -1.0] {
                        d3.draw_line3D(
                            Vector3::new(sx * x0, 0.02, sz * z0),
                            Vector3::new(sx * x1, 0.02, sz * z1),
                            Color::CYAN,
                        );
                    }
                }
            }

            draw_ball_collision_ring(d3, self.cue_ball_pos, Color::YELLOW);
            draw_ball_collision_ring(d3, self.object_ball_pos, Color::ORANGE);
        }

        if USE_MODEL_PROPS {
            // raylib always reserves materials()[0] for its own
            // auto-inserted default material (a blank white texture) and
            // appends the glTF file's real materials starting at index 1
            // -- balls.glb has exactly one ("Balls"), so index 1 is it.
            assets.balls_model.materials_mut()[1].set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, debug_tint);
            let balls_material = &assets.balls_model.materials()[1];
            let cue_ball_offset = self.cue_ball_pos - CUE_BALL_MODEL_CENTER;
            d3.draw_mesh(
                &assets.balls_model.meshes()[CUE_BALL_MESH_INDEX],
                weak_copy(balls_material),
                Matrix::translate(cue_ball_offset.x, cue_ball_offset.y, cue_ball_offset.z),
            );

            let red_ball_offset = self.object_ball_pos - RED_BALL_MODEL_CENTER;
            d3.draw_mesh(
                &assets.balls_model.meshes()[RED_BALL_MESH_INDEX],
                weak_copy(balls_material),
                Matrix::translate(red_ball_offset.x, red_ball_offset.y, red_ball_offset.z),
            );
        } else {
            assets.ball_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, CUE_BALL_COLOR);
            d3.draw_mesh(
                &assets.ball_mesh,
                weak_copy(&assets.ball_material),
                Matrix::translate(self.cue_ball_pos.x, self.cue_ball_pos.y, self.cue_ball_pos.z),
            );

            assets.ball_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, OBJECT_BALL_COLOR);
            d3.draw_mesh(
                &assets.ball_mesh,
                weak_copy(&assets.ball_material),
                Matrix::translate(self.object_ball_pos.x, self.object_ball_pos.y, self.object_ball_pos.z),
            );
        }

        if let Some(dir) = shot_dir {
            if USE_MODEL_PROPS {
                draw_cue_model(d3, &assets.cue_model, dir, self.cue_ball_pos, debug_tint);
            } else {
                draw_cue(d3, dir, self.cue_ball_pos);
            }

            if self.show_ghost_ball {
                let raycast = cue_raycast(dir, self.cue_ball_pos, self.object_ball_pos);
                assets.ghost_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, GHOST_BALL_COLOR);
                d3.draw_mesh(
                    &assets.ball_mesh,
                    weak_copy(&assets.ghost_material),
                    Matrix::translate(raycast.ghost_pos.x, raycast.ghost_pos.y, raycast.ghost_pos.z),
                );
                if self.show_aim_line && raycast.hit_object_ball {
                    draw_object_ball_aim_line(d3, raycast.ghost_pos, self.object_ball_pos);
                }
                if self.show_collision_debug {
                    draw_ball_collision_ring(d3, raycast.ghost_pos, Color::LIME);
                }
            }
        }

        let (pocket_idx, gate_dir) = self.target_pocket;
        let gate_color = match self.shot_test.as_ref().map(|s| &s.gate_state) {
            Some(GateState::Success) => GATE_SUCCESS_COLOR,
            Some(GateState::Miss) => GATE_MISS_COLOR,
            None => GATE_NEUTRAL_COLOR,
        };
        draw_gate(d3, assets.pockets[pocket_idx].position, gate_dir, gate_color);

        if let Some(test) = &self.shot_test {
            draw_path_stripe(d3, self.cue_ball_pos, test.white_end, PATH_WHITE_COLOR);
            if let Some((start, end)) = test.red_path {
                draw_path_stripe(d3, start, end, PATH_RED_COLOR);
                assets.ghost_material.set_map_color(MaterialMapIndex::MATERIAL_MAP_ALBEDO, GHOST_RED_BALL_COLOR);
                d3.draw_mesh(
                    &assets.ball_mesh,
                    weak_copy(&assets.ghost_material),
                    Matrix::translate(end.x, end.y, end.z),
                );
            }
        }
    }

    /// 2D overlay: FPS, the view-mode label, every on-screen control, and
    /// the help popup (when open).
    pub fn draw_overlay(
        &self,
        d: &mut RaylibDrawHandle,
        ui: &TouchUi,
        mouse: Vector2,
        screen_w: i32,
        screen_h: i32,
    ) {
        d.draw_fps(10, 10);
        if self.view_mode {
            d.draw_text("VIEW MODE (cue aim frozen)", 10, 36, 18, Color::YELLOW);
        }

        // On-screen touch controls -- drawn every frame, on every platform
        // (desktop mouse clicks work identically to touch taps, since
        // browsers synthesize mouse events from touch input).
        ui.pan_up.draw(d, mouse, false);
        ui.pan_down.draw(d, mouse, false);
        ui.pan_left.draw(d, mouse, false);
        ui.pan_right.draw(d, mouse, false);
        ui.rot_left.draw(d, mouse, false);
        ui.rot_right.draw(d, mouse, false);
        ui.zoom_in.draw(d, mouse, false);
        ui.zoom_out.draw(d, mouse, false);
        ui.reset.draw(d, mouse, false);
        ui.test.draw(d, mouse, false);
        if let Some(b) = &ui.ghost {
            b.draw(d, mouse, self.show_ghost_ball);
        }
        if let Some(b) = &ui.aim {
            b.draw(d, mouse, self.show_aim_line);
        }
        ui.close_stance.draw(d, mouse, false);
        ui.stand_stance.draw(d, mouse, false);
        ui.pot_line.draw(d, mouse, false);
        ui.expand_toggle.draw(d, mouse, false);
        ui.view.draw(d, mouse, self.view_mode);
        ui.center.draw(d, mouse, false);
        ui.help.draw(d, mouse, self.help_visible);

        if self.help_visible {
            d.draw_rectangle(0, 0, screen_w, screen_h, HELP_BG);

            let lines = [
                "Controls   (tap anywhere, or ? / the ? button, to close)",
                "",
                "Drag / D-pad     orbit the camera",
                "Scroll / +  -    zoom toward the cursor / camera target",
                "Pinch            zoom toward the point between your fingers",
                "WASD or arrows   pan the camera",
                "Q / E            rotate (yaw) the camera",
                "C / CENTER       re-center the orbit pivot on the cue ball",
                "V / LOOK         toggle view mode (inspect freely, aim stays frozen)",
                "1 / CLOSE        aim stance: right above and close to the cue ball",
                "2 / STAND        aim stance: standing back and to the left of the cue",
                "3 / LINE         view mode: sight the potting line, object ball to pocket",
                "G / GHOST        toggle the ghost cue ball",
                "H / AIM          toggle the object-ball aim line (needs ghost ball on)",
                ">> / <<          show or hide the GHOST/AIM buttons",
                "Space / HIT      test the current aim: trace both balls' paths",
                "R / CLEAR-NEXT   clear a tested shot, or reposition both balls",
                "? / help button  toggle this popup",
            ];
            let start_y = 90;
            let line_h = 26;
            for (i, line) in lines.iter().enumerate() {
                d.draw_text(line, 24, start_y + i as i32 * line_h, 20, Color::RAYWHITE);
            }
        }
    }
}
