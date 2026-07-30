"""
One-off preprocessing tool for assets/snooker_table.glb.

The source Sketchfab model (CC BY-NC, "Snooker table" by MesXwi:
https://sketchfab.com/3d-models/snooker-table-b5930e8c780a4eabaad4e3e9be23ef09)
ships with a full 22-ball rack, an overhead lamp, and a wall-mounted cue
rack (2 cues + case) baked into the scene as static geometry. None of
that is usable here -- this app draws its own 2 balls, ghost ball, and
cue procedurally every frame at dynamic positions, and raylib's GLTF
loader ignores glTF scenes entirely (see rmodels.c: "Scenes defined in
the glTF file are ignored. All nodes in the file are used"), so there is
no load-time way to hide unwanted nodes -- they have to be stripped from
the file itself, once, ahead of time.

This script removes those nodes/meshes/materials (identified by node
name and reindexed by hand against this specific file's structure -- the
indices below are NOT generic, they only apply to this exact source
file) and repacks a clean, table-only .glb. Re-run only if re-deriving
assets/snooker_table.glb from a fresh copy of the original download;
if the source model ever changes, the REMOVE_* index sets below will
need to be re-derived by inspecting the new file's node list.
"""

import struct, json, os

SRC = os.path.expanduser("~/Downloads/snooker_table.glb")
DST = os.path.join(os.path.dirname(__file__), "..", "assets", "snooker_table.glb")

with open(SRC, "rb") as f:
    magic, version, total_len = struct.unpack("<4sII", f.read(12))
    assert magic == b"glTF"
    chunk_len, chunk_type = struct.unpack("<II", f.read(8))
    assert chunk_type == 0x4E4F534A  # 'JSON'
    json_bytes = f.read(chunk_len)
    # binary chunk (BIN), if present -- passed through untouched
    bin_bytes = b""
    rest = f.read()
    if rest:
        bin_len, bin_type = struct.unpack("<II", rest[:8])
        assert bin_type == 0x004E4942  # 'BIN\0'
        bin_bytes = rest[8:8 + bin_len]

gltf = json.loads(json_bytes)
nodes = gltf["nodes"]
meshes = gltf["meshes"]
materials = gltf["materials"]

REMOVE_NODES = set(range(3, 19))    # cue-rack assembly: 2 cue-box panels + 6 cue sticks (paired wrapper+mesh nodes)
REMOVE_NODES |= set(range(19, 21))  # Lamp_8 + Object_20
REMOVE_NODES |= set(range(21, 66))  # the 22-ball hierarchy (7 individual balls + a 15-red group)
REMOVE_MESHES = set(range(0, 8))    # cue-rack meshes
REMOVE_MESHES |= {8}                # lamp mesh
REMOVE_MESHES |= set(range(9, 31))  # the 22 ball meshes
REMOVE_MATERIALS = {1, 2, 3, 4}     # "cue_box", "Cues", "Lamp", "Balls" -- unused once the above meshes are gone

assert nodes[3]["name"] == "Plane.006_0"
assert nodes[18]["name"] == "Object_18"
assert nodes[19]["name"] == "Lamp_8"
assert nodes[21]["name"] == "Black_9"
assert nodes[35]["name"] == "Empty_Red_triangle_31"
assert nodes[65]["name"] == "Object_65"
assert materials[1]["name"] == "cue_box"
assert materials[2]["name"] == "Cues"
assert materials[3]["name"] == "Lamp"
assert materials[4]["name"] == "Balls"
for i in REMOVE_MESHES:
    prims = meshes[i]["primitives"]
    assert all(p.get("material") in REMOVE_MATERIALS for p in prims), (i, prims)

def remap(old_index, removed_set):
    """New index for a surviving old_index, after removing everything in removed_set."""
    return old_index - sum(1 for r in removed_set if r < old_index)

# --- nodes ---
new_nodes = []
old_to_new_node = {}
for i, n in enumerate(nodes):
    if i in REMOVE_NODES:
        continue
    old_to_new_node[i] = len(new_nodes)
    new_nodes.append(n)

for n in new_nodes:
    if "children" in n and n["children"] is not None:
        n["children"] = [old_to_new_node[c] for c in n["children"] if c not in REMOVE_NODES]
        if not n["children"]:
            del n["children"]
    if "mesh" in n and n["mesh"] is not None:
        n["mesh"] = remap(n["mesh"], REMOVE_MESHES)

# --- meshes ---
new_meshes = [m for i, m in enumerate(meshes) if i not in REMOVE_MESHES]
for m in new_meshes:
    for p in m["primitives"]:
        if p.get("material") is not None:
            assert p["material"] not in REMOVE_MATERIALS
            p["material"] = remap(p["material"], REMOVE_MATERIALS)

# --- materials ---
new_materials = [mat for i, mat in enumerate(materials) if i not in REMOVE_MATERIALS]

# --- scene roots (top-level "scene" -> single Sketchfab_model node, untouched index 0) ---
gltf["scene"] = old_to_new_node.get(gltf["scene"], gltf["scene"])
for sc in gltf.get("scenes", []):
    sc["nodes"] = [old_to_new_node[n] for n in sc["nodes"] if n not in REMOVE_NODES]

gltf["nodes"] = new_nodes
gltf["meshes"] = new_meshes
gltf["materials"] = new_materials

print(f"nodes: {len(nodes)} -> {len(new_nodes)}")
print(f"meshes: {len(meshes)} -> {len(new_meshes)}")
print(f"materials: {len(materials)} -> {len(new_materials)}")

# --- repack as .glb ---
def pad(b, fill):
    n = (4 - len(b) % 4) % 4
    return b + fill * n

new_json_bytes = pad(json.dumps(gltf, separators=(",", ":")).encode("utf-8"), b" ")
new_bin_bytes = pad(bin_bytes, b"\x00")

os.makedirs(os.path.dirname(DST), exist_ok=True)
with open(DST, "wb") as f:
    total = 12 + 8 + len(new_json_bytes) + (8 + len(new_bin_bytes) if new_bin_bytes else 0)
    f.write(struct.pack("<4sII", b"glTF", 2, total))
    f.write(struct.pack("<II", len(new_json_bytes), 0x4E4F534A))
    f.write(new_json_bytes)
    if new_bin_bytes:
        f.write(struct.pack("<II", len(new_bin_bytes), 0x004E4942))
        f.write(new_bin_bytes)

print("wrote", DST, "-", os.path.getsize(DST), "bytes")
