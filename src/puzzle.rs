use raylib::prelude::Vector3;
use serde::Deserialize;

use crate::grid::GridSpec;
use crate::shot::{best_pocket, MAX_REACHABLE_CUT_DEG};
use crate::table::{ball_position_clear, Pocket, BALL_RADIUS, MAX_PLACEMENT_ATTEMPTS, MIN_BALL_SEPARATION};

pub const PUZZLES_DIR: &str = "assets/puzzles";
pub const GRID_PATH: &str = "assets/puzzles/grid.json";

/// An inclusive `[row_min, row_max]` / `[col_min, col_max]` range of grid
/// indices (see grid.rs / assets/puzzles/grid.json) a ball may be sampled
/// from. A fixed cell is just `min == max` on both axes.
#[derive(Deserialize, Clone, Copy)]
pub struct CellRange {
    pub row: [usize; 2],
    pub col: [usize; 2],
}

#[derive(Deserialize)]
pub struct Exercise {
    pub label: Option<String>,
    pub object_ball: CellRange,
    pub cue_ball: CellRange,
}

#[derive(Deserialize)]
pub struct PuzzleSet {
    pub id: String,
    pub name: String,
    // Not surfaced in the UI yet -- reserved for a future menu tooltip;
    // still part of the JSON schema since authored content should be able
    // to explain itself even before there's a place on screen to show it.
    #[allow(dead_code)]
    pub description: Option<String>,
    pub exercises: Vec<Exercise>,
}

#[derive(Deserialize)]
struct ManifestEntry {
    id: String,
    file: String,
}

#[derive(Deserialize)]
struct Manifest {
    sets: Vec<ManifestEntry>,
}

fn validate_range(range: CellRange, grid: &GridSpec, set_id: &str) {
    assert!(
        range.row[0] <= range.row[1] && range.row[1] < grid.rows.len(),
        "puzzle set '{set_id}': row range {:?} is out of bounds for a {}-row grid",
        range.row,
        grid.rows.len()
    );
    assert!(
        range.col[0] <= range.col[1] && range.col[1] < grid.cols.len(),
        "puzzle set '{set_id}': col range {:?} is out of bounds for a {}-col grid",
        range.col,
        grid.cols.len()
    );
}

/// Loads every puzzle set listed in `{dir}/manifest.json` (see puzzle set
/// JSON format in README.md), panicking on any missing/malformed file or
/// out-of-bounds grid reference -- same fail-fast style as `Assets::load`'s
/// `.expect()`s on the bundled `.glb` files, since this is bundled content,
/// not user input.
pub fn load_all(dir: &str, grid: &GridSpec) -> Vec<PuzzleSet> {
    let manifest_path = format!("{dir}/manifest.json");
    let manifest_str = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("failed to read {manifest_path}: {e}"));
    let manifest: Manifest = serde_json::from_str(&manifest_str)
        .unwrap_or_else(|e| panic!("failed to parse {manifest_path}: {e}"));

    manifest
        .sets
        .into_iter()
        .map(|entry| {
            let set_path = format!("{dir}/{}", entry.file);
            let set_str = std::fs::read_to_string(&set_path)
                .unwrap_or_else(|e| panic!("failed to read {set_path}: {e}"));
            let set: PuzzleSet = serde_json::from_str(&set_str)
                .unwrap_or_else(|e| panic!("failed to parse {set_path}: {e}"));
            assert_eq!(
                set.id, entry.id,
                "manifest entry id '{}' doesn't match {set_path}'s own id '{}'",
                entry.id, set.id
            );
            for exercise in &set.exercises {
                validate_range(exercise.object_ball, grid, &set.id);
                validate_range(exercise.cue_ball, grid, &set.id);
            }
            set
        })
        .collect()
}

fn sample_cell(range: CellRange) -> (usize, usize) {
    let row = rand::random_range(range.row[0]..=range.row[1]);
    let col = rand::random_range(range.col[0]..=range.col[1]);
    (row, col)
}

/// Samples a fresh (cue_ball_pos, object_ball_pos) pair for `exercise`:
/// a uniformly random grid cell within each of its ranges, rerolled (same
/// bounded-attempts idea as `random_shot_setup`) until both balls clear the
/// cushions/pockets, keep their minimum separation, and leave at least one
/// pocket reachable within `MAX_REACHABLE_CUT_DEG` -- deliberately the same
/// lenient ceiling `best_pocket` itself uses for "possible at all", not
/// free-random's stricter 65° threshold, since an exercise's range is a
/// deliberate authoring choice that may include a hard cut.
///
/// Panics if nothing ever qualifies -- a range that can *never* produce a
/// legal placement (most commonly: a single fixed cell that's inside a
/// cushion) is a content bug, not a runtime condition to paper over by
/// silently handing back an invalid position. Fail fast here the same way
/// `Assets::load` does for a missing/malformed asset file.
pub fn sample_exercise(exercise: &Exercise, grid: &GridSpec, pockets: &[Pocket]) -> (Vector3, Vector3) {
    let mut last_tried = (Vector3::zero(), Vector3::zero());
    for _ in 0..MAX_PLACEMENT_ATTEMPTS {
        let (or, oc) = sample_cell(exercise.object_ball);
        let (cr, cc) = sample_cell(exercise.cue_ball);
        let (ox, oz) = grid.world_pos(or, oc);
        let (cx, cz) = grid.world_pos(cr, cc);
        let object_pos = Vector3::new(ox, BALL_RADIUS, oz);
        let cue_pos = Vector3::new(cx, BALL_RADIUS, cz);
        last_tried = (cue_pos, object_pos);

        let positions_ok = ball_position_clear(object_pos, pockets)
            && ball_position_clear(cue_pos, pockets)
            && cue_pos.distance(object_pos) > MIN_BALL_SEPARATION;
        let (_, _, cut_angle) = best_pocket(pockets, cue_pos, object_pos);

        if positions_ok && cut_angle <= MAX_REACHABLE_CUT_DEG.to_radians() {
            return last_tried;
        }
    }
    panic!(
        "sample_exercise: no valid placement found for exercise {:?} after {MAX_PLACEMENT_ATTEMPTS} attempts \
         (last tried: cue={:?}, object={:?}) -- the range likely includes a cell too close to a cushion/pocket, \
         or too tight to ever clear separation",
        exercise.label, last_tried.0, last_tried.1
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::pockets;

    fn cell(v: usize) -> CellRange {
        CellRange { row: [v, v], col: [v, v] }
    }

    #[test]
    fn sampled_positions_land_within_the_requested_range_and_clear() {
        let grid = GridSpec::load(GRID_PATH);
        let pockets = pockets();
        // Row 32 / col 3 is the blue spot (z=0, x=0) in the bundled grid.
        let exercise = Exercise {
            label: None,
            object_ball: CellRange { row: [32, 32], col: [3, 3] },
            cue_ball: CellRange { row: [0, grid.rows.len() - 1], col: [0, grid.cols.len() - 1] },
        };

        for _ in 0..50 {
            let (cue_pos, object_pos) = sample_exercise(&exercise, &grid, &pockets);
            let (expected_x, expected_z) = grid.world_pos(32, 3);
            assert!((object_pos.x - expected_x).abs() < 1e-6 && (object_pos.z - expected_z).abs() < 1e-6);
            assert!(ball_position_clear(cue_pos, &pockets));
            assert!(cue_pos.distance(object_pos) > MIN_BALL_SEPARATION);
        }
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn out_of_bounds_range_is_rejected_at_load_time() {
        let grid = GridSpec::load(GRID_PATH);
        let bad_row = [0, grid.rows.len()];
        validate_range(CellRange { row: bad_row, col: [0, 0] }, &grid, "test");
    }

    #[test]
    fn single_cell_range_always_samples_the_same_cell() {
        let range = cell(0);
        for _ in 0..10 {
            assert_eq!(sample_cell(range), (0, 0));
        }
    }

    /// Loads the real bundled example content and samples every exercise a
    /// few times -- catches authoring mistakes (out-of-range cells, ranges
    /// too tight to ever clear separation, or a cell that's inside a
    /// cushion) that a hand-rolled test fixture wouldn't.
    #[test]
    fn bundled_example_sets_load_and_sample_cleanly() {
        let grid = GridSpec::load(GRID_PATH);
        let pockets = pockets();
        let sets = load_all(PUZZLES_DIR, &grid);
        assert_eq!(sets.len(), 3);

        for set in &sets {
            for exercise in &set.exercises {
                for _ in 0..20 {
                    let (cue_pos, object_pos) = sample_exercise(exercise, &grid, &pockets);
                    assert!(
                        cue_pos.distance(object_pos) > MIN_BALL_SEPARATION,
                        "set '{}' exercise {:?}: sampled balls too close",
                        set.id,
                        exercise.label
                    );
                    assert!(
                        ball_position_clear(cue_pos, &pockets),
                        "set '{}' exercise {:?}: cue ball not clear of cushions/pockets at {:?}",
                        set.id,
                        exercise.label,
                        cue_pos
                    );
                    assert!(
                        ball_position_clear(object_pos, &pockets),
                        "set '{}' exercise {:?}: object ball not clear of cushions/pockets at {:?}",
                        set.id,
                        exercise.label,
                        object_pos
                    );
                }
            }
        }
    }

    /// The Line-Up's 21 exercises are individual real positions (15 reds +
    /// 6 colours) along the drill's centre line -- verifies each one's
    /// object ball actually resolves to its exact intended real
    /// coordinate, not just "somewhere plausible".
    #[test]
    fn line_up_exercises_place_the_object_ball_at_the_exact_real_spot() {
        let grid = GridSpec::load(GRID_PATH);
        let pockets = pockets();
        let sets = load_all(PUZZLES_DIR, &grid);
        let line_up = sets.iter().find(|s| s.id == "line_up").expect("line_up must be bundled");
        assert_eq!(line_up.exercises.len(), 21);

        for exercise in &line_up.exercises {
            let (expected_x, expected_z) = grid.world_pos(exercise.object_ball.row[0], exercise.object_ball.col[0]);
            for _ in 0..10 {
                let (_, object_pos) = sample_exercise(exercise, &grid, &pockets);
                assert!(
                    (object_pos.x - expected_x).abs() < 1e-6 && (object_pos.z - expected_z).abs() < 1e-6,
                    "exercise {:?}: object ball at ({}, {}), expected ({expected_x}, {expected_z})",
                    exercise.label,
                    object_pos.x,
                    object_pos.z
                );
            }
        }
    }
}
