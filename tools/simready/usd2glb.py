"""Convert a SimReady USD asset (UsdPreviewSurface materials) to a single GLB.

Usage: python usd2glb.py <input.usd> <output.glb> [--tex 512]
Output is Y-up, metres, origin at bottom-centre of the bounding box.
"""
import io, json, os, struct, sys
import numpy as np
from PIL import Image
from pxr import Usd, UsdGeom, UsdShade, Gf

TEX = 512
if "--tex" in sys.argv:
    TEX = int(sys.argv[sys.argv.index("--tex") + 1])

src, dst = sys.argv[1], sys.argv[2]
stage = Usd.Stage.Open(src)
mpu = UsdGeom.GetStageMetersPerUnit(stage) or 1.0
zup = UsdGeom.GetStageUpAxis(stage) == UsdGeom.Tokens.z
xc = UsdGeom.XformCache(Usd.TimeCode.Default())


def shader_input(shader, name):
    """Return ('const', value) | ('tex', file_path, channel) | None."""
    inp = shader.GetInput(name)
    if not inp:
        return None
    if inp.HasConnectedSource():
        srcs = inp.GetConnectedSources()[0]
        if not srcs:
            return None
        s = srcs[0]
        up = UsdShade.Shader(s.source.GetPrim())
        f = up.GetInput("file")
        if f and f.Get():
            return ("tex", f.Get().resolvedPath or f.Get().path, str(s.sourceName))
        return None
    v = inp.Get()
    return None if v is None else ("const", v)


def read_material(mat):
    if not mat:
        return None
    surf = mat.ComputeSurfaceSource()[0]
    if not surf or surf.GetIdAttr().Get() != "UsdPreviewSurface":
        # find any UsdPreviewSurface inside
        surf = None
        for p in Usd.PrimRange(mat.GetPrim()):
            sh = UsdShade.Shader(p)
            if sh and sh.GetIdAttr().Get() == "UsdPreviewSurface":
                surf = sh
                break
        if not surf:
            return None
    return {
        "name": mat.GetPrim().GetName(),
        "diffuse": shader_input(surf, "diffuseColor"),
        "rough": shader_input(surf, "roughness"),
        "metal": shader_input(surf, "metallic"),
        "normal": shader_input(surf, "normal"),
        "opacity": shader_input(surf, "opacity"),
        "emissive": shader_input(surf, "emissiveColor"),
    }


def bound_material(prim):
    m, _ = UsdShade.MaterialBindingAPI(prim).ComputeBoundMaterial()
    return m


def to_corner(values, interp, counts, indices, n_points):
    """Expand a primvar to face-varying (one value per face corner)."""
    values = np.asarray(values)
    if interp in ("vertex", "varying"):
        return values[indices]
    if interp == "faceVarying":
        return values
    if interp == "uniform":
        return np.repeat(values, counts, axis=0)
    if interp == "constant":
        return np.repeat(values[:1], len(indices), axis=0)
    return None


# material key -> lists of positions/normals/uvs
groups = {}
materials = {}


def add(matprim, pos, nrm, uv):
    key = str(matprim.GetPath()) if matprim else "__default__"
    if key not in materials:
        materials[key] = read_material(matprim) if matprim else None
    groups.setdefault(key, []).append((pos, nrm, uv))


for prim in Usd.PrimRange(stage.GetPseudoRoot(), Usd.TraverseInstanceProxies()):
    if not prim.IsA(UsdGeom.Mesh):
        continue
    img = UsdGeom.Imageable(prim)
    if img.ComputeVisibility() == UsdGeom.Tokens.invisible:
        continue
    purpose = img.ComputePurpose()
    if purpose in (UsdGeom.Tokens.guide, UsdGeom.Tokens.proxy):
        continue
    mesh = UsdGeom.Mesh(prim)
    pts = mesh.GetPointsAttr().Get()
    counts = mesh.GetFaceVertexCountsAttr().Get()
    idx = mesh.GetFaceVertexIndicesAttr().Get()
    if not pts or not counts or not idx:
        continue
    pts = np.asarray(pts, dtype=np.float64)
    counts = np.asarray(counts)
    idx = np.asarray(idx)
    M = np.asarray(xc.GetLocalToWorldTransform(prim), dtype=np.float64)  # row-vector convention
    wpts = np.c_[pts, np.ones(len(pts))] @ M
    wpts = wpts[:, :3]

    # normals
    nrm_c = None
    pv_api = UsdGeom.PrimvarsAPI(prim)
    npv = pv_api.GetPrimvar("normals")
    if npv and npv.HasValue():
        nv = npv.ComputeFlattened()
        nrm_c = to_corner(nv, npv.GetInterpolation(), counts, idx, len(pts))
    elif mesh.GetNormalsAttr().HasValue():
        nrm_c = to_corner(mesh.GetNormalsAttr().Get(), mesh.GetNormalsInterpolation(), counts, idx, len(pts))
    if nrm_c is not None:
        N = np.linalg.inv(M[:3, :3]).T
        nrm_c = np.asarray(nrm_c, dtype=np.float64) @ N  # row vectors: n' = n · inv(M)ᵀ
    # uvs
    uv_c = None
    for nm in ("st", "st0", "UVMap", "uv", "map1"):
        pv = pv_api.GetPrimvar(nm)
        if pv and pv.HasValue():
            uv_c = to_corner(pv.ComputeFlattened(), pv.GetInterpolation(), counts, idx, len(pts))
            break

    # triangulate (fan) -> corner indices
    starts = np.r_[0, np.cumsum(counts)[:-1]]
    tri_corner = []
    face_of_tri = []
    for f, (s, c) in enumerate(zip(starts, counts)):
        for k in range(1, c - 1):
            tri_corner.append((s, s + k, s + k + 1))
            face_of_tri.append(f)
    if not tri_corner:
        continue
    tri_corner = np.asarray(tri_corner)
    face_of_tri = np.asarray(face_of_tri)

    if mesh.GetOrientationAttr().Get() == UsdGeom.Tokens.leftHanded:
        tri_corner = tri_corner[:, [0, 2, 1]]
    if np.linalg.det(M[:3, :3]) < 0:
        tri_corner = tri_corner[:, [0, 2, 1]]

    corner_pos = wpts[idx]

    def emit(tri_mask, matprim):
        tc = tri_corner[tri_mask].reshape(-1)
        if len(tc) == 0:
            return
        p = corner_pos[tc]
        n = nrm_c[tc] if nrm_c is not None and len(nrm_c) == len(idx) else None
        u = uv_c[tc] if uv_c is not None and len(uv_c) == len(idx) else None
        add(matprim, p, n, u)

    subsets = UsdGeom.Subset.GetAllGeomSubsets(mesh)
    subsets = [s for s in subsets if s.GetElementTypeAttr().Get() == UsdGeom.Tokens.face]
    if subsets:
        used = np.zeros(len(counts), bool)
        for s in subsets:
            faces = np.asarray(s.GetIndicesAttr().Get() or [], dtype=int)
            if len(faces) == 0:
                continue
            fm = np.zeros(len(counts), bool)
            fm[faces] = True
            used |= fm
            m = bound_material(s.GetPrim()) or bound_material(prim)
            emit(fm[face_of_tri], m)
        rest = ~used
        if rest.any():
            emit(rest[face_of_tri], bound_material(prim))
    else:
        emit(np.ones(len(face_of_tri), bool), bound_material(prim))

if not groups:
    sys.exit(f"no meshes in {src}")

# ---- assemble buffers ----
def fix_axes(a):
    a = a * mpu
    if zup:
        a = np.c_[a[:, 0], a[:, 2], -a[:, 1]]
    return a

allpos = np.concatenate([fix_axes(p) for g in groups.values() for (p, _, _) in g])
lo, hi = allpos.min(0), allpos.max(0)
offset = np.array([-(lo[0] + hi[0]) / 2, -lo[1], -(lo[2] + hi[2]) / 2])

bin_chunks = []
buffer_views, accessors, meshes_prims, images, textures, gl_materials = [], [], [], [], [], []
tex_cache = {}


def push(data: bytes, target=None):
    off = sum(len(c) for c in bin_chunks)
    pad = (-off) % 4
    if pad:
        bin_chunks.append(b"\0" * pad)
        off += pad
    bin_chunks.append(data)
    bv = {"buffer": 0, "byteOffset": off, "byteLength": len(data)}
    if target:
        bv["target"] = target
    buffer_views.append(bv)
    return len(buffer_views) - 1


def accessor(arr, comp, typ, target, minmax=False):
    bv = push(arr.tobytes(), target)
    a = {"bufferView": bv, "componentType": comp, "count": len(arr), "type": typ}
    if minmax:
        a["min"] = arr.min(0).tolist()
        a["max"] = arr.max(0).tolist()
    accessors.append(a)
    return len(accessors) - 1


def texture(path, channel_fmt=None, srgb=True):
    key = (path, channel_fmt)
    if key in tex_cache:
        return tex_cache[key]
    if not path or not os.path.exists(path):
        tex_cache[key] = None
        return None
    im = Image.open(path)
    im = im.convert("RGBA" if im.mode in ("RGBA", "LA", "P") else "RGB")
    if max(im.size) > TEX:
        im.thumbnail((TEX, TEX), Image.LANCZOS)
    has_alpha = im.mode == "RGBA" and np.asarray(im)[..., 3].min() < 250
    buf = io.BytesIO()
    if has_alpha:
        im.save(buf, "PNG", optimize=True)
        mime = "image/png"
    else:
        im.convert("RGB").save(buf, "JPEG", quality=85)
        mime = "image/jpeg"
    bv = push(buf.getvalue())
    images.append({"bufferView": bv, "mimeType": mime})
    textures.append({"source": len(images) - 1, "sampler": 0})
    tex_cache[key] = len(textures) - 1
    return tex_cache[key]


def make_material(m):
    pbr = {"baseColorFactor": [0.7, 0.7, 0.7, 1], "metallicFactor": 0.0, "roughnessFactor": 0.6}
    out = {"name": "default", "pbrMetallicRoughness": pbr, "doubleSided": False}
    if not m:
        return out
    out["name"] = m["name"]
    d = m["diffuse"]
    if d and d[0] == "const":
        pbr["baseColorFactor"] = [float(d[1][0]), float(d[1][1]), float(d[1][2]), 1]
    elif d and d[0] == "tex":
        t = texture(d[1])
        if t is not None:
            pbr["baseColorFactor"] = [1, 1, 1, 1]
            pbr["baseColorTexture"] = {"index": t}
    r, mt = m["rough"], m["metal"]
    if r and r[0] == "const":
        pbr["roughnessFactor"] = float(r[1])
    if mt and mt[0] == "const":
        pbr["metallicFactor"] = float(mt[1])
    # ORM packed texture: glTF expects G=roughness, B=metallic, which matches SimReady ORM
    orm = None
    if r and r[0] == "tex":
        orm = r[1]
    if mt and mt[0] == "tex":
        orm = orm or mt[1]
    if orm:
        t = texture(orm, "orm")
        if t is not None:
            pbr["metallicRoughnessTexture"] = {"index": t}
            pbr["roughnessFactor"] = 1.0
            pbr["metallicFactor"] = 1.0 if (mt and mt[0] == "tex") else pbr["metallicFactor"]
            if r and r[0] == "tex" and r[2] == "g" and (not mt or mt[0] != "tex" or mt[2] == "b"):
                out["occlusionTexture"] = {"index": t}
    n = m["normal"]
    if n and n[0] == "tex":
        t = texture(n[1], "nrm")
        if t is not None:
            out["normalTexture"] = {"index": t}
    o = m["opacity"]
    if o and o[0] == "const" and float(o[1]) < 0.99:
        pbr["baseColorFactor"][3] = float(o[1])
        out["alphaMode"] = "BLEND"
        out["doubleSided"] = True
    elif o and o[0] == "tex":
        out["alphaMode"] = "BLEND"
        out["doubleSided"] = True
    e = m["emissive"]
    if e and e[0] == "const" and max(e[1]) > 0:
        out["emissiveFactor"] = [float(x) for x in e[1]]
    return out


prims = []
for key, parts in groups.items():
    P = np.concatenate([fix_axes(p) for p, _, _ in parts]) + offset
    has_n = all(n is not None for _, n, _ in parts)
    has_uv = all(u is not None for _, _, u in parts)
    if has_n:
        Nn = np.concatenate([fix_axes(np.asarray(n, float) / mpu) for _, n, _ in parts])
    else:
        tri = P.reshape(-1, 3, 3)
        fn = np.cross(tri[:, 1] - tri[:, 0], tri[:, 2] - tri[:, 0])
        Nn = np.repeat(fn, 3, axis=0)
    ln = np.linalg.norm(Nn, axis=1, keepdims=True)
    ln[ln == 0] = 1
    Nn = Nn / ln
    attrs_list = [P.astype(np.float32), Nn.astype(np.float32)]
    if has_uv:
        U = np.concatenate([np.asarray(u, float) for _, _, u in parts])
        U = np.c_[U[:, 0], 1.0 - U[:, 1]]
        attrs_list.append(U.astype(np.float32))
    # weld identical corners
    stacked = np.concatenate([np.round(a.astype(np.float64), 5) for a in attrs_list], axis=1)
    uniq, inv = np.unique(stacked, axis=0, return_inverse=True)
    inv = inv.reshape(-1)
    first = np.zeros(len(uniq), dtype=np.int64)
    first[inv[::-1]] = np.arange(len(inv))[::-1]
    attrs_list = [a[first] for a in attrs_list]
    ind = inv.astype(np.uint32)
    attrs = {
        "POSITION": accessor(attrs_list[0], 5126, "VEC3", 34962, True),
        "NORMAL": accessor(attrs_list[1], 5126, "VEC3", 34962),
    }
    if has_uv:
        attrs["TEXCOORD_0"] = accessor(attrs_list[2], 5126, "VEC2", 34962)
    ia = accessor(ind, 5125, "SCALAR", 34963)
    gl_materials.append(make_material(materials[key]))
    prims.append({"attributes": attrs, "indices": ia, "material": len(gl_materials) - 1})

name = os.path.splitext(os.path.basename(dst))[0]
binblob = b"".join(bin_chunks)
binblob += b"\0" * ((-len(binblob)) % 4)
gltf = {
    "asset": {"version": "2.0", "generator": "usd2glb (SimReady)"},
    "scene": 0,
    "scenes": [{"nodes": [0]}],
    "nodes": [{"name": name, "mesh": 0}],
    "meshes": [{"name": name, "primitives": prims}],
    "materials": gl_materials,
    "accessors": accessors,
    "bufferViews": buffer_views,
    "buffers": [{"byteLength": len(binblob)}],
    "samplers": [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}],
}
if textures:
    gltf["textures"] = textures
    gltf["images"] = images
js = json.dumps(gltf, separators=(",", ":")).encode()
js += b" " * ((-len(js)) % 4)
with open(dst, "wb") as f:
    f.write(struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(binblob)))
    f.write(struct.pack("<II", len(js), 0x4E4F534A))
    f.write(js)
    f.write(struct.pack("<II", len(binblob), 0x004E4942))
    f.write(binblob)
size = hi - lo
print(json.dumps({"name": name, "size_m": [round(float(x), 3) for x in ((size[0], size[1], size[2]))],
                  "tris": int(sum(len(fix_axes(p)) for g in groups.values() for p, _, _ in g) // 3),
                  "materials": len(gl_materials), "bytes": os.path.getsize(dst)}))
