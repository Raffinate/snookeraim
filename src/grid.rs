use crate::table::{BALL_RADIUS, TABLE_LENGTH, TABLE_WIDTH};

// Official snooker table markings, in meters, derived from this project's
// own real tournament dimensions (see table.rs: TABLE_LENGTH/TABLE_WIDTH).
// Convention (arbitrary but fixed, like pocket_mouth_dir's sign choices in
// shot.rs): the baulk end is Z = -hl, the top end (where black/pink sit)
// is Z = +hl.
const BAULK_LINE_DIST: f32 = 0.737; // from the baulk cushion
const D_RADIUS: f32 = 0.292; // green/yellow sit this far either side of brown
const BLACK_SPOT_DIST_FROM_TOP: f32 = 0.324;

/// The six standard colour-ball spot positions, computed from the real
/// table dimensions rather than measured off a model -- these are fixed by
/// the rules of the game, not this table's particular geometry.
pub struct ColourSpots {
    pub baulk_z: f32,   // brown/green/yellow's Z (green/yellow also offset in X)
    pub green_x: f32,
    pub yellow_x: f32,
    pub center_z: f32,  // blue
    pub pyramid_z: f32, // pink -- midway between blue and the top cushion
    pub black_z: f32,
}

pub fn colour_spots() -> ColourSpots {
    let hl = TABLE_LENGTH / 2.0;
    ColourSpots {
        baulk_z: -hl + BAULK_LINE_DIST,
        green_x: -D_RADIUS,
        yellow_x: D_RADIUS,
        center_z: 0.0,
        pyramid_z: hl / 2.0,
        black_z: hl - BLACK_SPOT_DIST_FROM_TOP,
    }
}

// How many extra, evenly spaced grid lines to fill in alongside the anchor
// rows/cols, so cells exist across the whole table, not just at the six
// colour spots. First-cut numbers -- easy to retune, not load-bearing on
// anything else.
const FILLER_ROWS: usize = 10;
const FILLER_COLS: usize = 8;

// Anchor values this close together are treated as the same grid line
// (e.g. a filler value landing almost exactly on an anchor) -- the anchor's
// own exact value always wins, see `merge_dedup`.
const DEDUP_EPS: f32 = 0.01;

/// Combines `anchors` (kept exactly) with `filler` (discarded if within
/// `DEDUP_EPS` of something already present), then sorts ascending.
fn merge_dedup(anchors: &[f32], filler: &[f32]) -> Vec<f32> {
    let mut values: Vec<f32> = anchors.to_vec();
    for &f in filler {
        if !values.iter().any(|&v| (v - f).abs() < DEDUP_EPS) {
            values.push(f);
        }
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values
}

fn linspace(lo: f32, hi: f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| lo + (hi - lo) * i as f32 / (n - 1) as f32).collect()
}

/// Position of `target` in `values`, within `DEDUP_EPS` -- for locating an
/// anchor's index right after inserting it, so it's always found.
fn index_of(values: &[f32], target: f32) -> usize {
    values
        .iter()
        .position(|&v| (v - target).abs() < DEDUP_EPS)
        .expect("anchor value must be present in its own grid axis")
}

/// One shared table-wide grid: `rows` are Z coordinates (baulk to top),
/// `cols` are X coordinates (left to right), both ascending. The six
/// colour-spot anchors are exact members of these arrays; puzzle-set JSON
/// (see puzzle.rs) addresses cells by index into them. Indices aren't
/// fixed constants -- they depend on how many filler lines land where --
/// so each anchor's resolved index is stored alongside the axes.
pub struct GridSpec {
    pub rows: Vec<f32>,
    pub cols: Vec<f32>,
    // Not read by any runtime code path -- these exist so puzzle-set
    // authors (and this module's own tests) have a name for "which index
    // is the blue spot" instead of a bare magic number in JSON/test code.
    #[allow(dead_code)]
    pub row_baulk: usize,
    #[allow(dead_code)]
    pub row_center: usize,
    #[allow(dead_code)]
    pub row_pyramid: usize,
    #[allow(dead_code)]
    pub row_black: usize,
    #[allow(dead_code)]
    pub col_green: usize,
    #[allow(dead_code)]
    pub col_center: usize,
    #[allow(dead_code)]
    pub col_yellow: usize,
}

impl GridSpec {
    pub fn build() -> GridSpec {
        let spots = colour_spots();
        let hl = TABLE_LENGTH / 2.0;
        let hw = TABLE_WIDTH / 2.0;
        let margin = BALL_RADIUS * 2.0; // same outer margin as random_ball_position

        let row_anchors = [spots.baulk_z, spots.center_z, spots.pyramid_z, spots.black_z];
        let col_anchors = [spots.green_x, 0.0, spots.yellow_x];

        let row_filler = linspace(-hl + margin, hl - margin, FILLER_ROWS);
        let col_filler = linspace(-hw + margin, hw - margin, FILLER_COLS);

        let rows = merge_dedup(&row_anchors, &row_filler);
        let cols = merge_dedup(&col_anchors, &col_filler);

        GridSpec {
            row_baulk: index_of(&rows, spots.baulk_z),
            row_center: index_of(&rows, spots.center_z),
            row_pyramid: index_of(&rows, spots.pyramid_z),
            row_black: index_of(&rows, spots.black_z),
            col_green: index_of(&cols, spots.green_x),
            col_center: index_of(&cols, 0.0),
            col_yellow: index_of(&cols, spots.yellow_x),
            rows,
            cols,
        }
    }

    /// World (x, z) for a grid cell. `row`/`col` must be in range -- puzzle
    /// JSON is validated against `self.rows.len()`/`self.cols.len()` at
    /// load time (see puzzle.rs), so an out-of-range index here means bad
    /// content data and is left to panic rather than silently clamp.
    pub fn world_pos(&self, row: usize, col: usize) -> (f32, f32) {
        (self.cols[col], self.rows[row])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors_resolve_to_expected_real_world_spots() {
        let grid = GridSpec::build();
        let (x, z) = grid.world_pos(grid.row_center, grid.col_center);
        assert!((x - 0.0).abs() < 1e-6 && (z - 0.0).abs() < 1e-6, "blue spot should be table center");

        let (gx, gz) = grid.world_pos(grid.row_baulk, grid.col_green);
        let (yx, yz) = grid.world_pos(grid.row_baulk, grid.col_yellow);
        assert!((gz - yz).abs() < 1e-6, "green/yellow share the baulk row");
        assert!((yx - gx - 2.0 * D_RADIUS).abs() < 1e-4, "green/yellow are 2*D_RADIUS apart");

        let (_, pyramid_z) = grid.world_pos(grid.row_pyramid, grid.col_center);
        assert!(pyramid_z > 0.0 && pyramid_z < TABLE_LENGTH / 2.0, "pink sits between center and the top cushion");

        let (_, black_z) = grid.world_pos(grid.row_black, grid.col_center);
        assert!((black_z - (TABLE_LENGTH / 2.0 - BLACK_SPOT_DIST_FROM_TOP)).abs() < 1e-6);
    }

    #[test]
    fn axes_are_sorted_and_deduped() {
        let grid = GridSpec::build();
        for axis in [&grid.rows, &grid.cols] {
            for w in axis.windows(2) {
                assert!(w[1] - w[0] > DEDUP_EPS * 0.5, "axis must be strictly ascending, no near-duplicates");
            }
        }
    }
}
