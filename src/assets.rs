use raylib::prelude::*;

use crate::cue::CUE_MODEL_PATH;
use crate::shaders::{BALL_FS, BALL_VS, GHOST_FS, TABLE_FS};
use crate::table::{
    self, BALLS_MODEL_PATH, BALL_RADIUS, GALLERY_MODEL_PATH, LIGHT_COLOR_INTENSITY,
    LIGHT_PANEL_COUNT, SKY_MODEL_PATH, TABLE_MODEL_PATH,
};

/// Everything loaded once at startup and read-only afterward: GPU resources
/// (shaders/materials/models/mesh) plus the static scene data (pocket
/// layout, overhead light positions) that goes with them.
pub struct Assets {
    pub ball_mesh: Mesh,
    pub ball_shader: Shader,
    pub ghost_shader: Shader,
    pub table_shader: Shader,
    pub ball_material: WeakMaterial,
    pub ghost_material: WeakMaterial,
    pub table_model: Model,
    pub cue_model: Model,
    pub balls_model: Model,
    pub gallery_model: Model,
    pub sky_model: Model,
    view_pos_loc: i32,
    ghost_view_pos_loc: i32,
    table_view_pos_loc: i32,
    pub light_panels: [Vector3; LIGHT_PANEL_COUNT],
    pub pockets: Vec<table::Pocket>,
}

impl Assets {
    pub fn load(rl: &mut RaylibHandle, thread: &RaylibThread) -> Assets {
        let pockets = table::pockets();
        let light_panels = table::light_panel_centers();

        let ball_mesh = Mesh::gen_mesh_sphere(thread, BALL_RADIUS, 24, 24);
        let mut ball_shader = rl.load_shader_from_memory(thread, Some(BALL_VS), Some(BALL_FS));
        let ambient_loc = ball_shader.get_shader_location("ambient");
        let view_pos_loc = ball_shader.get_shader_location("viewPos");
        let light_pos_loc = ball_shader.get_shader_location("lightPos");
        let light_color_loc = ball_shader.get_shader_location("lightColor");

        ball_shader.set_shader_value(ambient_loc, Vector4::new(0.35, 0.35, 0.35, 1.0));
        ball_shader.set_shader_value_v(light_pos_loc, &light_panels);
        let light_colors = [Vector4::new(
            LIGHT_COLOR_INTENSITY,
            LIGHT_COLOR_INTENSITY,
            LIGHT_COLOR_INTENSITY,
            1.0,
        ); LIGHT_PANEL_COUNT];
        ball_shader.set_shader_value_v(light_color_loc, &light_colors);

        let mut ball_material = rl.load_material_default(thread);
        ball_material.set_shader(&ball_shader);

        let mut ghost_shader = rl.load_shader_from_memory(thread, Some(BALL_VS), Some(GHOST_FS));
        let ghost_ambient_loc = ghost_shader.get_shader_location("ambient");
        let ghost_view_pos_loc = ghost_shader.get_shader_location("viewPos");
        let ghost_light_pos_loc = ghost_shader.get_shader_location("lightPos");
        let ghost_light_color_loc = ghost_shader.get_shader_location("lightColor");
        ghost_shader.set_shader_value(ghost_ambient_loc, Vector4::new(0.35, 0.35, 0.35, 1.0));
        ghost_shader.set_shader_value_v(ghost_light_pos_loc, &light_panels);
        ghost_shader.set_shader_value_v(ghost_light_color_loc, &light_colors);

        let mut ghost_material = rl.load_material_default(thread);
        ghost_material.set_shader(&ghost_shader);

        let mut table_shader = rl.load_shader_from_memory(thread, Some(BALL_VS), Some(TABLE_FS));
        let table_ambient_loc = table_shader.get_shader_location("ambient");
        let table_view_pos_loc = table_shader.get_shader_location("viewPos");
        let table_light_pos_loc = table_shader.get_shader_location("lightPos");
        let table_light_color_loc = table_shader.get_shader_location("lightColor");
        table_shader.set_shader_value(table_ambient_loc, Vector4::new(0.35, 0.35, 0.35, 1.0));
        table_shader.set_shader_value_v(table_light_pos_loc, &light_panels);
        table_shader.set_shader_value_v(table_light_color_loc, &light_colors);

        let mut table_model = rl
            .load_model(thread, TABLE_MODEL_PATH)
            .expect("failed to load assets/snooker_table.glb");
        for material in table_model.materials_mut() {
            material.set_shader(&table_shader);
        }

        let mut cue_model = rl
            .load_model(thread, CUE_MODEL_PATH)
            .expect("failed to load assets/cue.glb");
        for material in cue_model.materials_mut() {
            material.set_shader(&table_shader);
        }

        let mut balls_model = rl
            .load_model(thread, BALLS_MODEL_PATH)
            .expect("failed to load assets/balls.glb");
        for material in balls_model.materials_mut() {
            material.set_shader(&table_shader);
        }

        let mut gallery_model = rl
            .load_model(thread, GALLERY_MODEL_PATH)
            .expect("failed to load assets/gallery.glb");
        for material in gallery_model.materials_mut() {
            material.set_shader(&table_shader);
        }

        // No shader assigned here (unlike the models above) -- a skybox
        // should render unlit, not respond to the table's overhead point
        // lights. (scripts/fix_sky_material.py patched this file's material
        // to use a standard pbrMetallicRoughness block so raylib's glTF
        // loader actually loads its texture -- see that script for why.)
        let sky_model = rl
            .load_model(thread, SKY_MODEL_PATH)
            .expect("failed to load assets/sky.glb");

        Assets {
            ball_mesh,
            ball_shader,
            ghost_shader,
            table_shader,
            ball_material,
            ghost_material,
            table_model,
            cue_model,
            balls_model,
            gallery_model,
            sky_model,
            view_pos_loc,
            ghost_view_pos_loc,
            table_view_pos_loc,
            light_panels,
            pockets,
        }
    }

    /// Section 6 of the old loop: pushes the camera position to all three
    /// shaders' viewPos uniform every frame.
    pub fn sync_view_pos(&mut self, camera_pos: Vector3) {
        self.ball_shader.set_shader_value(self.view_pos_loc, camera_pos);
        self.ghost_shader.set_shader_value(self.ghost_view_pos_loc, camera_pos);
        self.table_shader.set_shader_value(self.table_view_pos_loc, camera_pos);
    }
}
