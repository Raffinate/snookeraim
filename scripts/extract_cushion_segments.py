"""
Builds src/cushion_segments.rs -- the real cushion nose boundary (the
cloth-covered raised strip a ball's surface actually touches) as two
dense piecewise-linear tables, read directly from the table model's own
mesh vertices. No hand-fit numbers.

Key fact this script relies on (verified by direct inspection, see the
session that wrote this): the "Baize" nose mesh (node 19, mesh 8) has
full-length coverage along both the long rails and the short rails, not
just near pockets. Its vertices come in cross-section rings at several
heights (roughly y in [0, 0.04]).

Near each pocket mouth, TWO physically distinct surfaces both pass
through ball-contact height at the same position along the rail: the
actual sloped nose (curving from its straight-rail value out to the
flat cloth bed's edge right at the mouth) and a separate, nearly
vertical pocket-throat wall sitting at a roughly constant "outward"
coordinate across a wide height range. A ball travelling outward from
the table center meets whichever surface is closer, so for each
position along the rail we take the nearest (i.e. least-outward) of
the two, not just whichever vertex happens to be nearest ball-contact
height -- taking the nearest-height point indiscriminately can pick
the throat wall over the (closer, and therefore actually load-bearing)
nose curve and produces a spurious zigzag.

Earlier attempts here got the geometry backwards: the cloth bed's edge
(|x|=TABLE_WIDTH/2 on the long rails, |z|=TABLE_LENGTH/2 on the short
ones -- where the rail attaches) is NOT the nose. The nose recesses
inward from that by about 5.8cm along every straight run (confirmed the
same recess depth on both the long and short rails -- it's the same
cushion cross-section all the way around), and only flares back out
toward the bed edge in a window right at each pocket mouth (a real
cushion-facing/shoulder feature, so balls aren't guarded by the
straight cushion line right at the pocket).

Past that flare's peak nearest a corner, the mesh's raised surface
curves sharply back inward again before the rail's extent ends -- that
is the pocket *throat*, the inside of the hole itself, not a wall a
ball travelling along the cushion ever bounces off. Using it as a
collision boundary makes the ball stop at a phantom wall well short of
the real, visible cushion (confirmed: produced exactly that as a
visible gap in testing, on both rails). So this script drops that final
dip on each end and holds the boundary flat at the flare's peak value
from there out to the table edge -- physically, "no more cushion, this
is pocket" rather than "cushion recedes to a point."

Only the non-negative half of each table is written (both rails are
symmetric about their own center); main.rs's safe_half_width() and
safe_half_length() both take an abs_* coordinate so a half table is all
either needs.

Re-run this if assets/snooker_table.glb (or its extraction from the
source model in scripts/strip_table_model.py) ever changes.
"""

import struct, json, os
from collections import defaultdict

SRC = os.path.join(os.path.dirname(__file__), "..", "assets", "snooker_table.glb")
OUT = os.path.join(os.path.dirname(__file__), "..", "src", "cushion_segments.rs")

OFFSET = (-0.1, -0.8697, 1.879)
BALL_RADIUS = 0.02625
HEIGHT_BAND = 0.012  # +/- around BALL_RADIUS

TABLE_WIDTH = 1.778
TABLE_LENGTH = 3.569


def mat_mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def mat_vec(m, v):
    return [sum(m[i][k] * v[k] for k in range(4)) for i in range(4)]


def identity():
    return [[1.0 if i == j else 0.0 for j in range(4)] for i in range(4)]


def local_matrix(node):
    if "matrix" in node:
        m = node["matrix"]
        return [[m[c * 4 + r] for c in range(4)] for r in range(4)]
    t = node.get("translation", [0, 0, 0])
    r = node.get("rotation", [0, 0, 0, 1])
    s = node.get("scale", [1, 1, 1])
    x, y, z, w = r
    rot = [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w), 0],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w), 0],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y), 0],
        [0, 0, 0, 1],
    ]
    scale = [[s[0], 0, 0, 0], [0, s[1], 0, 0], [0, 0, s[2], 0], [0, 0, 0, 1]]
    trans = identity()
    trans[0][3], trans[1][3], trans[2][3] = t
    return mat_mul(mat_mul(trans, rot), scale)


with open(SRC, "rb") as f:
    magic, version, length = struct.unpack("<4sII", f.read(12))
    chunk_len, chunk_type = struct.unpack("<II", f.read(8))
    json_data = f.read(chunk_len)
    rest = f.read()
    bin_len, bin_type = struct.unpack("<II", rest[:8])
    bin_bytes = rest[8 : 8 + bin_len]

gltf = json.loads(json_data)
nodes, meshes, accessors, bvs = gltf["nodes"], gltf["meshes"], gltf["accessors"], gltf["bufferViews"]

child_ids = {c for n in nodes for c in (n.get("children") or [])}
roots = [i for i in range(len(nodes)) if i not in child_ids]
node_world = {}


def visit(i, parent):
    world = mat_mul(parent, local_matrix(nodes[i]))
    node_world[i] = world
    for c in nodes[i].get("children") or []:
        visit(c, world)


for r in roots:
    visit(r, identity())

# The Baize nose mesh: node 19 / mesh 8. (There is a second "Baize" mesh
# at node 44 / mesh 21 with a negative Y range -- that's the underside of
# the cloth bed, not the raised nose; not used here.)
NODE_IDX, MESH_IDX = 19, 8
prim = meshes[MESH_IDX]["primitives"][0]
world = node_world[NODE_IDX]
pos_acc = accessors[prim["attributes"]["POSITION"]]
bv = bvs[pos_acc["bufferView"]]
stride = bv.get("byteStride", 12)
base = bv.get("byteOffset", 0) + pos_acc.get("byteOffset", 0)

verts = []
for i in range(pos_acc["count"]):
    off = base + i * stride
    x, y, z = struct.unpack_from("<fff", bin_bytes, off)
    wx, wy, wz, _ = mat_vec(world, [x, y, z, 1.0])
    verts.append((wx + OFFSET[0], wy + OFFSET[1], wz + OFFSET[2]))


def filter_monotonic(points, rising, tol=0.004):
    # tol lets a plateau (near the flare's peak, a few mm of mesh noise
    # around an otherwise flat maximum) stay in rather than getting
    # chopped down to a single point by a strict, no-tolerance compare.
    kept = []
    best = None
    for along, out in points:
        if best is None:
            kept.append((along, out))
            best = out
            continue
        ok = (out >= best - tol) if rising else (out <= best + tol)
        if ok:
            kept.append((along, out))
            best = max(best, out) if rising else min(best, out)
    return kept


def extract_boundary(along_idx, out_idx, mouth_zone, corner_zone, table_edge):
    """along_idx/out_idx pick which world axis (0=x,2=z) is "along the
    rail" and which is "outward, toward the cushion" for this rail."""
    raised = [v for v in verts if v[1] > 0.01 and v[out_idx] > 0.5]

    groups = defaultdict(list)
    for v in raised:
        if abs(v[1] - BALL_RADIUS) > HEIGHT_BAND:
            continue
        groups[round(v[along_idx], 3)].append(v[out_idx])

    pts = []
    for along, outs in groups.items():
        if along < 0:
            continue  # half table only; caller uses an abs_* coordinate
        pts.append((along, min(outs)))  # nearer of any overlapping surfaces
    pts.sort()

    zoned = set()
    cleaned = []
    for lo, hi, rising in (mouth_zone, corner_zone):
        zone_pts = sorted(p for p in pts if lo <= p[0] < hi)
        zoned.update(zone_pts)
        cleaned.extend(filter_monotonic(zone_pts, rising))
    corner_peak_start = corner_zone[0]
    cleaned.extend(p for p in pts if p not in zoned and p[0] < corner_peak_start)

    pts = sorted(cleaned)
    peak_out = max(out for along, out in pts if along >= corner_zone[0])
    pts.append((table_edge, peak_out))
    return pts


# Long rails: boundary as a function of Z (x = 0), flaring near the
# middle pocket (z~0) and each corner (z~TABLE_LENGTH/2).
long_rail = extract_boundary(
    along_idx=2, out_idx=0,
    mouth_zone=(0.0, 0.20, False),
    corner_zone=(1.55, 1.715, True),
    table_edge=TABLE_LENGTH / 2,
)

# Short rails: boundary as a function of X (z = 0), flaring only near
# each corner (x~TABLE_WIDTH/2) -- no middle pocket on a short rail.
short_rail = extract_boundary(
    along_idx=0, out_idx=2,
    mouth_zone=(-1.0, -1.0, False),  # no middle-pocket-equivalent zone
    corner_zone=(0.55, 0.80, True),
    table_edge=TABLE_WIDTH / 2,
)

with open(OUT, "w") as f:
    f.write(
        "// Generated by scripts/extract_cushion_segments.py -- do not edit by hand.\n"
        "// Real cushion nose boundaries, read from the table model's own \"Baize\"\n"
        "// nose mesh vertices at ball-center height (see that script's docstring).\n"
        "// Both cover only the non-negative half of their rail (each is symmetric\n"
        "// about its own center); safe_half_width()/safe_half_length() below take\n"
        "// an abs_* coordinate. Piecewise linear between points.\n"
        "\n"
        "// Long rails (left/right): [z, abs_x].\n"
    )
    f.write("const CUSHION_BOUNDARY: &[[f32; 2]] = &[\n")
    for along, out in long_rail:
        f.write(f"    [{along:.4}, {out:.4}],\n")
    f.write("];\n\n")
    f.write("// Short rails (baulk/top): [x, abs_z].\n")
    f.write("const SHORT_RAIL_BOUNDARY: &[[f32; 2]] = &[\n")
    for along, out in short_rail:
        f.write(f"    [{along:.4}, {out:.4}],\n")
    f.write("];\n")

print(f"wrote {OUT}: long_rail {len(long_rail)} pts (z 0..{long_rail[-1][0]:.4f}), "
      f"short_rail {len(short_rail)} pts (x 0..{short_rail[-1][0]:.4f})")
