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

# --- drop now-orphaned textures/images (used by nothing in new_materials) ---
# Removing nodes/meshes/materials above only unhooks JSON *references* --
# the underlying image bytes stay in the binary buffer unless we also trim
# the buffer itself, so this second pass physically cuts them out.
textures = gltf["textures"]
images = gltf["images"]
buffer_views = gltf["bufferViews"]

used_tex = set()
for m in new_materials:
    pbr = m.get("pbrMetallicRoughness", {})
    for key in ("baseColorTexture", "metallicRoughnessTexture"):
        if key in pbr:
            used_tex.add(pbr[key]["index"])
    for key in ("normalTexture", "occlusionTexture", "emissiveTexture"):
        if key in m:
            used_tex.add(m[key]["index"])
used_img = {textures[t]["source"] for t in used_tex}
remove_img = set(range(len(images))) - used_img
remove_bv_img = {images[i]["bufferView"] for i in remove_img}

# Physically cut the removed images' bytes out of the binary buffer. They
# were verified contiguous (a single run of bufferViews, all in one
# buffer, with no accessor pointing into that range) via manual inspection
# of this specific file -- re-verify if the source model ever changes.
assert all(buffer_views[bv]["buffer"] == 0 for bv in remove_bv_img)
cut_ranges = sorted(
    (buffer_views[bv]["byteOffset"], buffer_views[bv]["byteOffset"] + buffer_views[bv]["byteLength"])
    for bv in remove_bv_img
)
for (s1, e1), (s2, e2) in zip(cut_ranges, cut_ranges[1:]):
    assert e1 == s2, "removed image bufferViews are not contiguous -- cannot safely cut bytes"
cut_start, cut_end = cut_ranges[0][0], cut_ranges[-1][1]
assert all(a.get("bufferView") not in remove_bv_img for a in gltf["accessors"])

new_bin_bytes_raw = bin_bytes[:cut_start] + bin_bytes[cut_end:]
cut_len = cut_end - cut_start

def shift_offset(bv):
    offset = bv.get("byteOffset", 0)
    if offset >= cut_end:
        bv["byteOffset"] = offset - cut_len

remaining_bvs = [bv for i, bv in enumerate(buffer_views) if i not in remove_bv_img]
for bv in remaining_bvs:
    shift_offset(bv)

old_to_new_bv = {}
n = 0
for i in range(len(buffer_views)):
    if i in remove_bv_img:
        continue
    old_to_new_bv[i] = n
    n += 1

for a in gltf["accessors"]:
    if a.get("bufferView") is not None:
        a["bufferView"] = old_to_new_bv[a["bufferView"]]

new_images = []
old_to_new_img = {}
for i, im in enumerate(images):
    if i in remove_img:
        continue
    old_to_new_img[i] = len(new_images)
    im["bufferView"] = old_to_new_bv[im["bufferView"]]
    new_images.append(im)

new_textures = []
old_to_new_tex = {}
for i, tex in enumerate(textures):
    if i not in used_tex:
        continue
    old_to_new_tex[i] = len(new_textures)
    tex["source"] = old_to_new_img[tex["source"]]
    new_textures.append(tex)

for m in new_materials:
    pbr = m.get("pbrMetallicRoughness", {})
    for key in ("baseColorTexture", "metallicRoughnessTexture"):
        if key in pbr:
            pbr[key]["index"] = old_to_new_tex[pbr[key]["index"]]
    for key in ("normalTexture", "occlusionTexture", "emissiveTexture"):
        if key in m:
            m[key]["index"] = old_to_new_tex[m[key]["index"]]

gltf["bufferViews"] = remaining_bvs
gltf["images"] = new_images
gltf["textures"] = new_textures
gltf["buffers"][0]["byteLength"] = len(new_bin_bytes_raw)
bin_bytes = new_bin_bytes_raw

print(f"images: {len(images)} -> {len(new_images)} (reclaimed {cut_len} bytes)")

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
