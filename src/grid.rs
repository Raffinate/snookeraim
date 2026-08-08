use serde::Deserialize;

/// The shared table-wide grid puzzle-set JSON addresses cells on (see
/// puzzle.rs). Purely data, loaded from `assets/puzzles/grid.json` --
/// nothing here computes a position; every row/col value in that file is
/// a real-world meter coordinate a human chose and wrote down directly
/// (X = table width, Z = table length, origin at table center).
#[derive(Deserialize)]
pub struct GridSpec {
    pub rows: Vec<f32>,
    pub cols: Vec<f32>,
}

impl GridSpec {
    pub fn load(path: &str) -> GridSpec {
        let contents = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        serde_json::from_str(&contents).unwrap_or_else(|e| panic!("failed to parse {path}: {e}"))
    }

    /// World (x, z) for a grid cell. `row`/`col` must be in range -- puzzle
    /// JSON is validated against `self.rows.len()`/`self.cols.len()` at
    /// load time (see puzzle.rs), so an out-of-range index here means bad
    /// content data and is left to panic rather than silently clamp.
    pub fn world_pos(&self, row: usize, col: usize) -> (f32, f32) {
        (self.cols[col], self.rows[row])
    }
}
