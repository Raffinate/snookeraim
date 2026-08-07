"""
One-off preprocessing tool for assets/sky.glb.

The downloaded model ("free_-_skybox_anime_sky", Sketchfab) exports its one
material using the KHR_materials_pbrSpecularGlossiness extension for its
base color instead of a standard pbrMetallicRoughness block. raylib's glTF
loader (rmodels.c LoadGLTF) only loads *any* texture for a material --
including its emissive texture, which this material also references at the
top level -- when materials[i].has_pbr_metallic_roughness is true; every
texture-loading branch, emissive included, is nested inside that one `if`
(rmodels.c:5531). Since this material has no such block, raylib silently
loaded zero textures for it and the skybox rendered as a solid black/white
sphere.

This adds a standard pbrMetallicRoughness.baseColorTexture pointing at the
same embedded image (texture index 0, still JPEG -- see Cargo.toml's
SUPPORT_FILEFORMAT_JPG feature for why raylib can decode it); nothing else
in the file changes.

Re-run only if re-deriving assets/sky.glb from a fresh copy of the
original download.
"""

import struct, json, os

SRC = os.path.expanduser("~/Downloads/free_-_skybox_anime_sky.glb")
DST = os.path.join(os.path.dirname(__file__), "..", "assets", "sky.glb")

with open(SRC, "rb") as f:
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

gltf = json.loads(json_bytes)
materials = gltf["materials"]
assert len(materials) == 1, "script assumes the single-material file this was written against"
m = materials[0]
assert "pbrMetallicRoughness" not in m, "material already has a standard PBR block -- nothing to fix"
assert "KHR_materials_pbrSpecularGlossiness" in m.get("extensions", {})

m["pbrMetallicRoughness"] = {
    "baseColorTexture": {"index": 0},
    "metallicFactor": 0.0,
    "roughnessFactor": 1.0,
}

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
