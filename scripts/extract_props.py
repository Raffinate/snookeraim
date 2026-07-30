"""
Extracts the cue-rack and ball geometry (+ their own textures) out of the
original Sketchfab source model into two standalone, self-contained .glb
files -- assets/cue.glb and assets/balls.glb -- so they can be loaded and
drawn independently of the table (see scripts/strip_table_model.py, which
does the inverse: keep everything *except* these subtrees).

Like strip_table_model.py, the root node indices below are hand-derived
against this specific source file's structure by inspecting its node
list; they are not generic and would need re-deriving from a different
model.
"""

import struct, json, os

SRC = os.path.expanduser("~/Downloads/snooker_table.glb")
OUT_DIR = os.path.join(os.path.dirname(__file__), "..", "assets")

# Top-level (direct children of GLTF_SceneRootNode) roots for each prop set.
# Root 5 is the actual full-length cue: 1.486m as a single continuous
# mesh, matching a real cue almost exactly. Confirmed by sampling actual
# vertex radii along its length (not just the bounding box) -- tapers
# smoothly from ~13mm at local -X down to ~4mm at local +X, i.e. -X is
# the butt end, +X is the tip. (Root 9, tried first, turned out to be a
# short ~0.6m 2-piece rest/extension, not the cue -- its two-node
# structure was a red herring for "assembled"; nodes 3/17 are the case
# panels, and 7/13/15 are the other loose cue/extensions -- all excluded.)
CUE_ROOTS = [5]
BALL_ROOTS = [21, 23, 25, 27, 29, 31, 33, 35]  # 6 colors + cue ball + the 15-red group


def load_glb(path):
    with open(path, "rb") as f:
        magic, version, total_len = struct.unpack("<4sII", f.read(12))
        assert magic == b"glTF"
        chunk_len, chunk_type = struct.unpack("<II", f.read(8))
        assert chunk_type == 0x4E4F534A  # 'JSON'
        json_bytes = f.read(chunk_len)
        bin_bytes = b""
        rest = f.read()
        if rest:
            bin_len, bin_type = struct.unpack("<II", rest[:8])
            assert bin_type == 0x004E4942  # 'BIN\0'
            bin_bytes = rest[8:8 + bin_len]
    return json.loads(json_bytes), bin_bytes


def save_glb(path, gltf, bin_bytes):
    def pad(b, fill):
        n = (4 - len(b) % 4) % 4
        return b + fill * n

    json_bytes = pad(json.dumps(gltf, separators=(",", ":")).encode("utf-8"), b" ")
    bin_bytes = pad(bin_bytes, b"\x00")

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        total = 12 + 8 + len(json_bytes) + (8 + len(bin_bytes) if bin_bytes else 0)
        f.write(struct.pack("<4sII", b"glTF", 2, total))
        f.write(struct.pack("<II", len(json_bytes), 0x4E4F534A))
        f.write(json_bytes)
        if bin_bytes:
            f.write(struct.pack("<II", len(bin_bytes), 0x004E4942))
            f.write(bin_bytes)


def extract_subset(gltf, bin_bytes, root_node_indices):
    """Returns a new (gltf, bin_bytes) containing only the nodes reachable
    from root_node_indices, plus everything they transitively reference
    (meshes -> materials -> textures -> images, and mesh accessors),
    reindexed and with the binary buffer rebuilt to hold only the
    retained bytes."""
    nodes = gltf["nodes"]
    meshes = gltf["meshes"]
    materials = gltf["materials"]
    accessors = gltf["accessors"]
    buffer_views = gltf["bufferViews"]
    textures = gltf["textures"]
    images = gltf["images"]

    keep_nodes = set()
    def visit(i):
        if i in keep_nodes:
            return
        keep_nodes.add(i)
        for c in nodes[i].get("children") or []:
            visit(c)
    for r in root_node_indices:
        visit(r)

    keep_meshes = {nodes[i]["mesh"] for i in keep_nodes if nodes[i].get("mesh") is not None}

    keep_materials = set()
    for mi in keep_meshes:
        for p in meshes[mi]["primitives"]:
            if p.get("material") is not None:
                keep_materials.add(p["material"])

    keep_tex = set()
    for mi in keep_materials:
        m = materials[mi]
        pbr = m.get("pbrMetallicRoughness", {})
        for key in ("baseColorTexture", "metallicRoughnessTexture"):
            if key in pbr:
                keep_tex.add(pbr[key]["index"])
        for key in ("normalTexture", "occlusionTexture", "emissiveTexture"):
            if key in m:
                keep_tex.add(m[key]["index"])

    keep_img = {textures[t]["source"] for t in keep_tex}

    keep_accessors = set()
    for mi in keep_meshes:
        for p in meshes[mi]["primitives"]:
            for v in p.get("attributes", {}).values():
                keep_accessors.add(v)
            if p.get("indices") is not None:
                keep_accessors.add(p["indices"])

    # Several accessors across *many different meshes* point into the same
    # handful of large bufferViews (one shared position/normal/texcoord/
    # index pool per attribute type, each accessor just slicing in via its
    # own byteOffset) -- copying a "needed" bufferView wholesale, as a
    # naive port would, drags in every other mesh's data too. Instead,
    # give each kept accessor (and each kept image) its own dedicated,
    # tightly-sliced bufferView containing only its own bytes.
    COMPONENT_SIZES = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
    TYPE_COMPONENTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT2": 4, "MAT3": 9, "MAT4": 16}

    new_buf = bytearray()
    new_bv_list = []

    def append_slice(data):
        while len(new_buf) % 4 != 0:
            new_buf.append(0)
        offset = len(new_buf)
        new_buf.extend(data)
        return offset

    def add_accessor_bufferview(acc):
        bv = buffer_views[acc["bufferView"]]
        elem_size = COMPONENT_SIZES[acc["componentType"]] * TYPE_COMPONENTS[acc["type"]]
        stride = bv.get("byteStride", elem_size)
        base = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
        if stride == elem_size:
            data = bin_bytes[base: base + acc["count"] * elem_size]
            stride_out = None
        else:
            # Genuinely interleaved with other attributes -- preserve the
            # stride pattern rather than just the tail element's bytes.
            data = bin_bytes[base: base + (acc["count"] - 1) * stride + elem_size]
            stride_out = stride
        offset = append_slice(data)
        new_bv = {"buffer": 0, "byteOffset": offset, "byteLength": len(data)}
        if stride_out is not None:
            new_bv["byteStride"] = stride_out
        if bv.get("target") is not None:
            new_bv["target"] = bv["target"]
        new_bv_list.append(new_bv)
        return len(new_bv_list) - 1

    def add_image_bufferview(old_img_idx):
        bv = buffer_views[images[old_img_idx]["bufferView"]]
        off = bv.get("byteOffset", 0)
        data = bin_bytes[off: off + bv["byteLength"]]
        offset = append_slice(data)
        new_bv_list.append({"buffer": 0, "byteOffset": offset, "byteLength": len(data)})
        return len(new_bv_list) - 1

    ordered_nodes = sorted(keep_nodes)
    old_to_new_node = {old: i for i, old in enumerate(ordered_nodes)}
    ordered_meshes = sorted(keep_meshes)
    old_to_new_mesh = {old: i for i, old in enumerate(ordered_meshes)}
    ordered_materials = sorted(keep_materials)
    old_to_new_mat = {old: i for i, old in enumerate(ordered_materials)}
    ordered_img = sorted(keep_img)
    old_to_new_img = {old: i for i, old in enumerate(ordered_img)}
    ordered_tex = sorted(keep_tex)
    old_to_new_tex = {old: i for i, old in enumerate(ordered_tex)}
    ordered_acc = sorted(keep_accessors)
    old_to_new_acc = {old: i for i, old in enumerate(ordered_acc)}

    new_nodes = []
    for old in ordered_nodes:
        n = json.loads(json.dumps(nodes[old]))
        if "children" in n and n["children"] is not None:
            n["children"] = [old_to_new_node[c] for c in n["children"] if c in keep_nodes]
            if not n["children"]:
                del n["children"]
        if n.get("mesh") is not None:
            n["mesh"] = old_to_new_mesh[n["mesh"]]
        new_nodes.append(n)

    new_meshes = []
    for old in ordered_meshes:
        m = json.loads(json.dumps(meshes[old]))
        for p in m["primitives"]:
            for k in list(p.get("attributes", {}).keys()):
                p["attributes"][k] = old_to_new_acc[p["attributes"][k]]
            if p.get("indices") is not None:
                p["indices"] = old_to_new_acc[p["indices"]]
            if p.get("material") is not None:
                p["material"] = old_to_new_mat[p["material"]]
        new_meshes.append(m)

    new_materials = []
    for old in ordered_materials:
        m = json.loads(json.dumps(materials[old]))
        pbr = m.get("pbrMetallicRoughness", {})
        for key in ("baseColorTexture", "metallicRoughnessTexture"):
            if key in pbr:
                pbr[key]["index"] = old_to_new_tex[pbr[key]["index"]]
        for key in ("normalTexture", "occlusionTexture", "emissiveTexture"):
            if key in m:
                m[key]["index"] = old_to_new_tex[m[key]["index"]]
        new_materials.append(m)

    new_images = []
    for old in ordered_img:
        im = json.loads(json.dumps(images[old]))
        im["bufferView"] = add_image_bufferview(old)
        new_images.append(im)

    new_textures = []
    for old in ordered_tex:
        t = json.loads(json.dumps(textures[old]))
        t["source"] = old_to_new_img[t["source"]]
        new_textures.append(t)

    new_accessors = []
    for old in ordered_acc:
        a = json.loads(json.dumps(accessors[old]))
        if a.get("bufferView") is not None:
            a["bufferView"] = add_accessor_bufferview(accessors[old])
            a.pop("byteOffset", None)  # now 0 -- start of its own dedicated bufferView
        new_accessors.append(a)

    new_gltf = {
        "asset": gltf["asset"],
        "scene": 0,
        "scenes": [{"nodes": [old_to_new_node[r] for r in root_node_indices]}],
        "nodes": new_nodes,
        "meshes": new_meshes,
        "materials": new_materials,
        "accessors": new_accessors,
        "bufferViews": new_bv_list,
        "textures": new_textures,
        "images": new_images,
        "samplers": gltf.get("samplers", []),
        "buffers": [{"byteLength": len(new_buf)}],
    }
    return new_gltf, bytes(new_buf)


def flatten_root(gltf, bin_bytes):
    """Extracted single-prop files (cue, etc.) come from a display-rack
    pose: the root wrapper node has a baked rotation+translation (posing
    it wherever it sat in the original scene). Rather than undo a 3D
    rotation matrix in Rust (easy to get a sign or axis wrong with no way
    to see the result), bake it out here instead: drop straight to the
    root's pre-rotation local frame by deleting its matrix, so its local
    +X axis becomes whatever the mesh's own geometry naturally lies
    along -- for something like the cue, that's its own long axis.

    Handles one extra wrinkle if present: a second mesh nested one level
    down under a translation-only wrapper (e.g. a second piece of a
    multi-part prop) -- since dropping the root's matrix would otherwise
    leave that piece positioned relative to a transform that no longer
    exists, its translation gets baked directly into its vertices first
    (translation never affects normals, so those are untouched).
    """
    nodes = gltf["nodes"]
    meshes = gltf["meshes"]
    accessors = gltf["accessors"]
    buffer_views = gltf["bufferViews"]

    assert "matrix" in nodes[0]
    buf = bytearray(bin_bytes)

    for child in nodes[0].get("children", []):
        n = nodes[child]
        if n.get("mesh") is not None:
            continue  # direct mesh child -- no offset needed once root's matrix is gone
        assert "matrix" in n, f"unexpected childless-of-root node {child} with neither mesh nor matrix"
        assert n["matrix"][:12] == [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0], "expected translation-only nested wrapper"
        offset = n["matrix"][12:15]

        piece_node = n["children"][0]
        mesh = meshes[nodes[piece_node]["mesh"]]
        acc = accessors[mesh["primitives"][0]["attributes"]["POSITION"]]
        bv = buffer_views[acc["bufferView"]]
        assert acc["componentType"] == 5126 and acc["type"] == "VEC3"  # FLOAT VEC3
        stride = bv.get("byteStride", 12)
        base = bv.get("byteOffset", 0) + acc.get("byteOffset", 0)
        for i in range(acc["count"]):
            off = base + i * stride
            x, y, z = struct.unpack_from("<fff", buf, off)
            struct.pack_into("<fff", buf, off, x + offset[0], y + offset[1], z + offset[2])
        acc["min"] = [acc["min"][k] + offset[k] for k in range(3)]
        acc["max"] = [acc["max"][k] + offset[k] for k in range(3)]

        del n["matrix"]

    del nodes[0]["matrix"]

    return gltf, bytes(buf)


if __name__ == "__main__":
    gltf, bin_bytes = load_glb(SRC)

    cue_gltf, cue_bin = extract_subset(gltf, bin_bytes, CUE_ROOTS)
    cue_gltf, cue_bin = flatten_root(cue_gltf, cue_bin)
    save_glb(os.path.join(OUT_DIR, "cue.glb"), cue_gltf, cue_bin)
    print(f"cue.glb: {len(cue_gltf['nodes'])} nodes, {len(cue_gltf['meshes'])} meshes, "
          f"{len(cue_gltf['materials'])} materials, {os.path.getsize(os.path.join(OUT_DIR, 'cue.glb'))} bytes")

    balls_gltf, balls_bin = extract_subset(gltf, bin_bytes, BALL_ROOTS)
    save_glb(os.path.join(OUT_DIR, "balls.glb"), balls_gltf, balls_bin)
    print(f"balls.glb: {len(balls_gltf['nodes'])} nodes, {len(balls_gltf['meshes'])} meshes, "
          f"{len(balls_gltf['materials'])} materials, {os.path.getsize(os.path.join(OUT_DIR, 'balls.glb'))} bytes")
