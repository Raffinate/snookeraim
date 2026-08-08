use raylib::prelude::*;

mod shaders;
mod touch_ui;
use touch_ui::{menu_ui, touch_ui};
mod camera;
use camera::shot_direction_xz;
mod table;
mod cue;
mod shot;
mod cushion_segments;
mod assets;
use assets::Assets;
mod game_state;
use game_state::GameState;

fn main() {
    let (mut rl, thread) = raylib::init()
        .size(1280, 800)
        .title("Snooker Aim Trainer")
        .msaa_4x()
        .resizable()
        .build();

    rl.set_target_fps(60);
    // Esc is claimed by the pause menu instead (see GameState::handle_menu)
    // -- without this, raylib's default exit-on-Esc would close the window
    // out from under the menu the moment it opened.
    rl.set_exit_key(None);

    let mut assets = Assets::load(&mut rl, &thread);

    let mut state = GameState::new(&assets.pockets);

    while !rl.window_should_close() && !state.quit_requested {
        let mouse = rl.get_mouse_position();
        let screen_w = rl.get_screen_width();
        let screen_h = rl.get_screen_height();
        let reset_label = if state.shot_test.is_some() { "CLEAR" } else { "NEXT" };
        let ui = touch_ui(screen_w, screen_h, reset_label, state.ghost_aim_visible);
        let menu = menu_ui(screen_w, screen_h);
        let tap = rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT);
        let held = rl.is_mouse_button_down(MouseButton::MOUSE_BUTTON_LEFT);
        let over_ui = ui.hit_any(mouse);

        state.handle_global_toggles(&rl, &ui, mouse, tap);
        state.handle_menu(&menu, mouse, tap);

        if !state.help_visible && !state.menu_visible {
            state.handle_reset_and_toggles(&rl, &ui, mouse, tap, &assets.pockets);
            state.handle_camera_presets(&rl, &ui, mouse, tap, &assets.pockets);
        }

        // This is derived from state, not input, so it's still computed
        // while the help popup or pause menu is open (the frozen 3D scene
        // keeps rendering behind it).
        let shot_dir = shot_direction_xz(state.aim_camera());

        if !state.help_visible && !state.menu_visible {
            state.maybe_test_shot(&rl, &ui, mouse, tap, shot_dir, &assets.pockets);
            state.handle_camera_movement(&rl, &ui, mouse, held, tap, over_ui, shot_dir, &assets.pockets);
        }

        assets.sync_view_pos(state.camera.position);

        // Needs `rl` for the world-to-screen projection, so this has to
        // happen before `begin_drawing` starts borrowing it exclusively for
        // the rest of the frame's drawing.
        let pot_marker = state.shot_result_marker(&rl);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::new(30, 30, 30, 255));

        {
            let mut d3 = d.begin_mode3D(state.camera);
            state.draw_scene(&mut d3, &mut assets, shot_dir);
        }

        state.draw_overlay(&mut d, &ui, &menu, mouse, screen_w, screen_h, pot_marker);
    }
}
