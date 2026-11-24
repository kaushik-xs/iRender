# Draw-call reduction, LOD and culling for three.js digital twins (40-200 repeated glTF objects, some moving)

Context: today each object is a deep clone of a glTF scene (one clone = several meshes = several draw calls), with per-object material clones for highlight/alarm glow, raycast picking, and some objects moved along paths every frame. Research done Nov 2026 against current three.js docs/source (r16x-r18x era).

## 1. InstancedMesh vs BatchedMesh vs mergeGeometries: draw calls, updates, highlight, picking, culling, multi-material glTF

### Takeaway
For a kit with ~40-200 objects built from a handful of glTF asset types, one InstancedMesh per (asset type x glTF primitive/material) is the best default: draw calls drop from (objects x primitives) to (asset types x primitives), moving instances are cheap (one matrix write + one buffer upload), highlight is per-instance colour, and raycasts return `instanceId`. BatchedMesh is for many *distinct* geometries sharing a material and has documented CPU-side per-frame costs (per-instance culling + sorting + texture uploads); merging static geometry is cheapest to draw but loses per-object control. At only 40-200 objects the draw-call count is probably not catastrophic even today, so profile first.

### Cited Findings
**InstancedMesh**
- Each instance's local transform lives in `instanceMatrix` (an InstancedBufferAttribute); after `setMatrixAt` you must set `instanceMatrix.needsUpdate = true`; same for `instanceColor` after `setColorAt` — [three.js docs: InstancedMesh](https://threejs.org/docs/pages/InstancedMesh.html)
- The engine computes the bounding sphere automatically for raycasting/frustum culling, but "you may need to recompute the bounding sphere if an instance is transformed via setMatrixAt"; bounding box is never computed automatically — [three.js docs: InstancedMesh](https://threejs.org/docs/pages/InstancedMesh.html)
- Source: the bounding sphere is ONE sphere that is the union of every instance's transformed geometry sphere (`boundingSphere.makeEmpty()` ... `union(_sphere)`), so frustum culling and the raycast early-out are for the whole InstancedMesh, not per instance — [InstancedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/InstancedMesh.js)
- Raycast: early-out `if (raycaster.ray.intersectsSphere(_sphere) === false) return;`, then loops over every instance up to `count` and tags hits with `intersect.instanceId = instanceId` — [InstancedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/InstancedMesh.js)
- Forum advice: one InstancedMesh per geometry type; per-instance transform via `instanceMatrix`, colour via `instanceColor`; raycasting returns `instanceId` for picking (HEOJUNFO); donmccurdy: both InstancedMesh and BatchedMesh give each instance its own position, rotation, scale, colour — [forum: many objects low fps](https://discourse.threejs.org/t/many-objects-caused-low-fps-can-we-optimize-it-without-losing-control-over-individual-mesh/76837)
- Historical caveat: older versions required `instanceColor` to be filled for the full instance count (informal, historical) — [same thread](https://discourse.threejs.org/t/many-objects-caused-low-fps-can-we-optimize-it-without-losing-control-over-individual-mesh/76837)
- A forum maintainer said InstancedMesh is often better for repeated geometry whose per-instance transforms change in real time, and BatchedMesh is designed for static or rarely-updated geometry — [forum: BatchedMesh batch ID](https://discourse.threejs.org/t/batchedmesh-batch-id/91306) (via search snippet)
- Guidance by geometry count (Fennec): one geometry -> InstancedMesh; 2-4 geometries with many instances -> one InstancedMesh per geometry; many geometries -> BatchedMesh — [forum: how to choose](https://discourse.threejs.org/t/how-to-choose-between-instancedmesh-and-batchedmesh/81221)
- agargaro: BatchedMesh uses `WEBGL_multi_draw`, InstancedMesh uses instanced drawing; multi-draw performs worse above ~100k instances but suits many distinct geometries; with few geometry types InstancedMesh performs better — [forum: how to choose](https://discourse.threejs.org/t/how-to-choose-between-instancedmesh-and-batchedmesh/81221)

**BatchedMesh**
- "renders many objects that share one material with multi-draw batching"; constructor `new BatchedMesh(maxInstanceCount, maxVertexCount, maxIndexCount, material)`; API `addGeometry`, `addInstance`, `setMatrixAt`, `setColorAt` (Vector4 for alpha), `setVisibleAt`, `setGeometryIdAt` (swap geometry per instance — usable for LOD), `deleteInstance`, `optimize()` (repacks after deletes), `setGeometrySize`, `setInstanceCount`, `setCustomSort` — [three.js docs: BatchedMesh](https://threejs.org/docs/pages/BatchedMesh.html)
- `perObjectFrustumCulled` (default true) culls individual objects; `sortObjects` (default true) sorts front-to-back for opaque, back-to-front for transparent — [three.js docs: BatchedMesh](https://threejs.org/docs/pages/BatchedMesh.html)
- Limits: "The first geometry added defines the layout of the batch. All subsequent geometries must match its index usage and provide its attributes with the same itemSize and normalized values"; negatively-scaled matrices not supported; bounds are null until computed and must be recomputed when instances change — [three.js docs: BatchedMesh](https://threejs.org/docs/pages/BatchedMesh.html)
- Source: matrices are stored in a float RGBA DataTexture (16 floats/instance), colours in a lazily-created Float32 RGBA DataTexture, plus a Uint32 `_indirectTexture` mapping draw slot -> instance — [BatchedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/BatchedMesh.js)
- Source: on WebGL, `onBeforeRender` -> `_updateDrawParameters` loops over ALL instances each frame, transforms each cached geometry bounding sphere by the instance matrix and tests `frustum.intersectsSphere`; it only early-returns when visibility hasn't changed AND culling and sorting are both disabled. On the WebGPU path this loop is skipped (only flags the indirect texture) — [BatchedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/BatchedMesh.js)
- Source: `raycast()` loops over every visible instance, sets a helper mesh to that geometry's draw range and instance matrix, calls standard `Mesh.raycast`, and tags hits with `intersect.batchId = i` — [BatchedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/BatchedMesh.js)
- Source: material arrays are accepted by the JSDoc type but culling/sorting reads single-material props (`material.transparent`, `material.wireframe`); per-group materials not handled — [BatchedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/BatchedMesh.js)
- Issue #28776 (open, "Needs Investigation"): Revit model ~100k sub-geometries: BatchedMesh ~20 fps / 40% CPU vs plain Mesh ~60 fps; gkjohnson's cube demo: InstancedMesh and merged geometry ~120 fps vs BatchedMesh ~30 fps on M1 Pro; his guess: per-frame upload of draw "starts"/"counts" buffers (~1.6 MB for 200k items); "without indirect draw support (WebGPU) BatchedMesh is being pushed too far". nkallen: per-BatchedMesh sorting and frustum culling in onBeforeRender dominate CPU. Enabling `sortObjects` alone dropped one test from 30 to 9 fps (texSubImage2D cost). QuisMagni: BatchedMesh 30-50% slower than merged geometry with ~6,000 geometries. Workaround: disable `sortObjects` and `perObjectFrustumCulled` when not needed — [three.js issue #28776](https://github.com/mrdoob/three.js/issues/28776)
- Forum thread on same: BatchedMesh ~40% CPU/~20 fps vs Mesh ~15% CPU/~60 fps on ~12M triangle model; same draw-call count in both versions; got worse after r166 — [forum: performance drop with BatchedMesh](https://discourse.threejs.org/t/significant-performance-drop-and-high-cpu-usage-with-batchedmesh/67324)
- gkjohnson: BatchedMesh's multi-draw supports per-geometry sorting, culling, visibility and instancing without duplicating geometry; merging everything into one mesh loses these; he noted a WebGL multidraw performance bottleneck tracked in a Chromium issue — [forum: when is LOD beneficial](https://discourse.threejs.org/t/when-is-it-actually-beneficial-to-use-lod-in-three-js-for-performance/87697)
- BatchedMesh relies on the multi_draw extension and falls back to a loop when missing, so its benefit is hardware-dependent — [forum: emulate BatchedMesh with InstancedMeshes](https://discourse.threejs.org/t/can-i-emulate-a-batchedmesh-with-instancedmeshes-that-share-a-material/95853) (via search snippet)

**Merging (BufferGeometryUtils.mergeGeometries / gltf-transform)**
- Merged geometry can cost more than it saves if the extra vertices drawn offset the draw-call gains — [forum: BatchMesh memory issue](https://discourse.threejs.org/t/batchmesh-memory-issue/29952) (via search snippet)
- Harold: a few thousand separate meshes slow down; one merged object per material performs well. donmccurdy (2025): ~6,700 meshes is "too many draw calls, and a large scene graph to traverse"; merge with gltf-transform and use BatchedMesh for interactive parts. makc3d: keep selection on a merged mesh by storing an object ID in a vertex attribute — [forum: performance of scene traversal](https://discourse.threejs.org/t/performance-of-scene-traversal/28537)
- donmccurdy on highlight with batching: hide the affected object in the batch and render it separately; or manage shared vertex/index buffers and use `geometry.groups` for hover/highlight — [forum: when is LOD beneficial](https://discourse.threejs.org/t/when-is-it-actually-beneficial-to-use-lod-in-three-js-for-performance/87697)

**Multi-material glTF / EXT_mesh_gpu_instancing**
- glTF Transform `instance()` creates GPU instances (EXT_mesh_gpu_instancing) for meshes referenced more than once; run `dedup()` first; "improves performance in many cases, but not all" — [gltf-transform instance()](https://gltf-transform.dev/modules/functions/functions/instance)
- GLTFLoader builds ordinary `THREE.InstancedMesh` objects for files using EXT_mesh_gpu_instancing; the loader's class builds each InstancedMesh from one geometry + one material + count (so a multi-primitive mesh yields one InstancedMesh per primitive — inference from quoted code) — [forum: GLTFLoader with InstancedMesh](https://discourse.threejs.org/t/the-problem-of-gltfloader-with-instancedmesh/44609), [forum: EXT_mesh_gpu_instancing multiple geometries](https://discourse.threejs.org/t/ext-mesh-gpu-instancing-not-supporting-multiple-geometries/49611)

### Inferences
- Draw-call arithmetic for the kit: today ~200 objects x (say) 5 primitives = ~1,000 draw calls (+ shadow pass doubles). With one InstancedMesh per (asset type, primitive), 8 asset types x 5 primitives = ~40 draw calls regardless of object count. BatchedMesh could collapse further to one call per *material* across asset types, but only if geometries share attribute layout and material — rarely true for arbitrary glTFs.
- Moving objects on paths: InstancedMesh cost per frame = N x `setMatrixAt` (16 float copies) + one `bufferSubData` of the whole instanceMatrix (200 x 64 B = 12.8 KB) — trivial. Remember to recompute (or pad) the bounding sphere, otherwise moving instances can be wrongly frustum-culled or missed by raycasts (the early-out sphere is stale). Simplest: `frustumCulled = false` on InstancedMeshes containing movers, or compute a generous sphere covering the path once.
- The kit's per-node animation (e.g. a rotating part inside a machine glTF) breaks naive instancing: a sub-part with its own animated transform must be its own InstancedMesh whose instance matrix = objectMatrix x partLocalMatrix, updated per frame. Skinned/morph-animated assets should stay as clones (or use InstancedMesh2 skinning).
- Hybrid is likely best: instance static/repeated assets; keep a small number of "special" objects (selected, alarm, animated) as regular clones — this is the "hide in batch, render separately" pattern donmccurdy describes.
- `DynamicDrawUsage`: not documented on the InstancedMesh page; setting `instanceMatrix.setUsage(THREE.DynamicDrawUsage)` is a standard hint for per-frame-updated buffers, but its measured effect is driver-dependent (no source found).

### Gaps
- No authoritative benchmark found for InstancedMesh vs clones at the 40-200 object scale.
- Default `usage` of `instanceMatrix` and measured impact of DynamicDrawUsage were not confirmed from source/docs.
- Whether BatchedMesh supports material arrays in practice is unclear from source (looks unsupported for groups).
- The Chromium multidraw bug status was not looked up.

## 2. Material sharing and texture atlases; highlight without cloning

### Takeaway
Cloning materials per object multiplies material state (uniform uploads, state switches) and kills instancing/batching (one material per draw is required). Highlight/alarm should instead be per-instance colour or a per-instance attribute/texture read in a patched shader, or a selection overlay (render the selected object separately / outline pass limited to the selection).

### Cited Findings
- discoverthreejs tips: "Always try to reuse objects such as objects, materials, textures etc."; materials with morph targets/skinning can't be shared; page warns "Not all the tips here have been experimentally verified" (written for older three.js: Geometry, gammaFactor, physicallyCorrectLights advice is outdated) — [Discover three.js tips](https://discoverthreejs.com/tips-and-tricks/)
- Each unique geometry or material usually needs its own draw call — [Codrops: Three.js Instances (2025-07-10)](https://tympanus.net/codrops/2025/07/10/three-js-instances-rendering-multiple-objects-simultaneously/) (via search snippet)
- three.js reuses shader programs across materials if GLSL is identical; `customProgramCacheKey()` is needed only when `onBeforeCompile` patches conditionally; `onBeforeCompile` and `customProgramCacheKey` are NOT carried by `.clone()`/`.copy()`/`.toJSON()` — [forum: defines vs customProgramCacheKey](https://discourse.threejs.org/t/defines-vs-customprogramcachekey/43711), [forum: onBeforeCompile problem](https://discourse.threejs.org/t/material-material-onbeforecompile-problem-for-three-js/53340) (via search snippets)
- three.js docs now recommend WebGPURenderer + Node materials/TSL for material customization; `customProgramCacheKey` behaviour applies to WebGLRenderer only — [Material docs mirror](https://syntax-scribe.github.io/sample-three.js/src/materials/Material) (via search snippet)
- Per-instance data can be stored in a texture and read by instance ID; works with merged, instanced and batched meshes (dubois) — [forum: many objects low fps](https://discourse.threejs.org/t/many-objects-caused-low-fps-can-we-optimize-it-without-losing-control-over-individual-mesh/76837)
- nkallen (#28776): for scenes with 100+ materials, pack material uniforms into a texture — [three.js issue #28776](https://github.com/mrdoob/three.js/issues/28776)
- InstancedMesh2 offers per-instance uniforms (`initUniformsPerInstance`, `setUniformAt`) with any material, plus per-instance opacity and visibility — [agargaro/instanced-mesh README](https://github.com/agargaro/instanced-mesh)
- OutlinePass: described on the forum as very inefficient with many objects; a perf drop after r116 was traced to all objects being included by default (pass only selected objects); downsample ratio default 2; TSL `OutlineNode` exists for WebGPU; for an instance, render that one instance as a regular Mesh and feed it to the outline — [forum: OutlinePass performance](https://discourse.threejs.org/t/performance-issues-with-three-outlinepass/7363), [OutlinePass docs](https://threejs.org/docs/pages/OutlinePass.html), [OutlineNode docs](https://threejs.org/docs/pages/OutlineNode.html), [forum: highlight edge of single instance](https://discourse.threejs.org/t/how-to-add-a-highlighted-edge-effect-to-a-single-target-of-an-instantiated-mesh-model/90419) (via search snippets)
- Overlapping instances with setColorAt appearing not to change was attributed to depth/render order — [forum: how to choose](https://discourse.threejs.org/t/how-to-choose-between-instancedmesh-and-batchedmesh/81221)

### Inferences
- Note instance colour *multiplies* the base colour (it is not emissive), so an "alarm glow" via `instanceColor` will tint, not glow. For a glow: add a per-instance float attribute (e.g. `instanceEmissive`) and patch `emissive` via `onBeforeCompile` (WebGL) or a TSL node on `emissiveNode` reading an instanced attribute (WebGPU). One shared material per asset part keeps one program and one draw call.
- Pulsing alarm: a single shared `uTime` uniform + per-instance alarm flag animates every alarmed object with zero per-frame JS per object.
- Material clones of identical built-in materials normally reuse the same compiled program (same GLSL), so the cost of cloning is mostly lost batching + extra per-object uniform uploads/state, not shader compiles — unless clones differ in defines (e.g. toggling a map/transparent), which creates new program variants and can hitch on first highlight.
- Texture atlases: only needed if you want to batch different-textured assets into one BatchedMesh/merged mesh; for InstancedMesh-per-asset, each asset keeps its own textures. No specific atlas source was gathered.

### Gaps
- No benchmark of clone-per-object materials vs shared material at this scale.
- No benchmark of stencil-outline vs OutlinePass.

## 3. LOD: THREE.LOD, MSFT_lod, simplification, impostors, InstancedMesh2, hysteresis

### Takeaway
LOD reduces triangles/vertex work, not draw calls, and costs per-object per-frame distance checks; for a 40-200 object twin it is a late optimisation unless assets are heavy CAD exports (hundreds of k triangles each). Pre-generate 2-3 LODs offline with gltf-transform/meshoptimizer, use THREE.LOD's built-in `hysteresis` (now a parameter of `addLevel`) to stop popping, or InstancedMesh2's `addLOD` to keep instancing.

### Cited Findings
- `THREE.LOD`: `autoUpdate` (default true; renderer updates it per frame; if false call `lod.update(camera)`), `levels: Array<{object, distance, hysteresis}>`, `addLevel(object, distance, hysteresis)` — hysteresis is "Threshold used to avoid flickering at LOD boundaries, as a fraction of distance", default 0; `getCurrentLevel()` — [three.js docs: LOD](https://threejs.org/docs/pages/LOD.html)
- manthrax: LOD addresses triangle count, which becomes a bottleneck around several million triangles/frame; it doesn't reduce draw calls; most apps with thousands of elements are draw-call bound. Recommended order: (1) reduce draw calls (merge/instance), (2) reduce triangles (LOD), (3) reduce per-frame JS (avoid per-frame raycasts; three-mesh-bvh), (4) overdraw, (5) shaders, (6) post-processing; merging+instancing alone "could yield a 10x-100x improvement". torrent: LOD levels resident in GPU memory don't hurt; per-mesh per-frame distance + swap overhead grows with LOD object count. mjurczyk: LOD is usually late-stage; impostors are a last resort (complex); render distant cylinders as lines. OP saw LOD make perf slightly worse with runtime-generated LODs; pre-generating is better — [forum: when is LOD beneficial](https://discourse.threejs.org/t/when-is-it-actually-beneficial-to-use-lod-in-three-js-for-performance/87697)
- gltf-transform `simplify({simplifier: MeshoptSimplifier, ratio, error})`: ratio 0 = max simplification, 0.5 ~ 50%; error caps deviation and stops early; sample `ratio: 0.75, error: 0.001`; run `weld()` first — split vertices limit the simplifier — [gltf-transform simplify](https://gltf-transform.dev/modules/functions/functions/simplify)
- meshoptimizer: example target 20% of indices with error 1e-2 (= <1% of mesh extents); not guaranteed to reach target; target count 0 = simplify as far as the error allows (useful for distance LODs); `simplifySloppy` ignores topology (worse quality); attribute-aware simplification with weights (normals ~1.0, UVs 10-100); permissive mode "usually results in higher quality LODs for the same target triangle count"; build LOD chains from the previous LOD for smoother transitions; use `meshopt_simplifyScale` to convert error to object space and pick LOD by screen-space error (`e * lod_factor >= lod_error`) — [meshoptimizer README](https://github.com/zeux/meshoptimizer)
- InstancedMesh2: `addLOD(geometry, material, distance)`, `addShadowLOD(geometry, distance)`, per-instance frustum culling on by default, `computeBVH({margin})` for culling/raycast ("works very well if the instances are mostly static"), sorting with `createRadixSort`, skinning, per-instance uniforms; no benchmark numbers on README (demos: "1kk static trees", "Skinning 3k instances") — [agargaro/instanced-mesh](https://github.com/agargaro/instanced-mesh)
- agargaro: LODs are easy with InstancedMesh2; with BatchedMesh they need his separate batched-mesh-extensions — [forum: how to choose](https://discourse.threejs.org/t/how-to-choose-between-instancedmesh-and-batchedmesh/81221)
- BatchedMesh `setGeometryIdAt(instanceId, geometryId)` lets you swap an instance's geometry (basis for LOD within a batch) — [three.js docs: BatchedMesh](https://threejs.org/docs/pages/BatchedMesh.html)

### Inferences
- With THREE.LOD per object you multiply scene-graph nodes and lose instancing; prefer LOD *per instance group*: one InstancedMesh per LOD level, and each frame (or every few frames / on camera move) reassign instances among levels by distance — this is what InstancedMesh2 does internally.
- Quality: error-bounded simplification (error ~0.001-0.01 relative) plus screen-space-error selection keeps popping invisible; hysteresis 0.1-0.2 of the switch distance is a reasonable starting point (no source for specific value).
- Since iRender twins are inspection views with the camera often close, LOD gains are smaller than for open worlds; simplifying the *source* assets once (LOD0 itself) is often the bigger win.

### Gaps
- MSFT_lod: three.js GLTFLoader support was not verified in this pass (I believe it is not supported natively — unconfirmed).
- No sourced number for recommended hysteresis values or impostor implementations in three.js.

## 4. Culling: frustum, BVH, occlusion, screen-size

### Takeaway
three.js default frustum culling is one bounding-sphere test per renderable object per frame — cheap at hundreds of objects. Converting to InstancedMesh makes culling all-or-nothing per InstancedMesh (fine for a twin where most objects are on screen). Per-instance culling exists in BatchedMesh (O(n) loop per frame on WebGL) and InstancedMesh2 (optional BVH). Occlusion culling is available only via WebGPURenderer occlusion queries; not worth it at this scale except in walled-in factory interiors.

### Cited Findings
- InstancedMesh culls on one union sphere of all instances — [InstancedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/InstancedMesh.js)
- BatchedMesh per-object culling: O(instances) CPU loop each frame on WebGL; skipped on WebGPU (indirect draw) — [BatchedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/BatchedMesh.js)
- InstancedMesh2: per-instance culling on by default; without a BVH checks every instance; BVH speeds culling and raycasting, best for mostly-static scattered instances — [agargaro/instanced-mesh](https://github.com/agargaro/instanced-mesh)
- Occlusion: three.js offers occlusion queries through WebGPURenderer (`webgpu_occlusion` example, `renderer.isOccluded(obj)` evaluated inside a node's update during render); a query requires rendering something (the object or a proxy bounding volume); a 2025 forum post says occlusion culling is only provided for the WebGPU renderer, and the WebGL2 query PR was closed — [forum: isOccluded returns null](https://discourse.threejs.org/t/renderer-isoccluded-returns-null-is-my-setup-correct/88356), [forum: preventing fully occluded objects](https://discourse.threejs.org/t/how-to-prevent-fully-occluded-objects-from-being-rendered-to-reduce-triangle-count-in-three-js/80071) (via search snippets)
- WebGPU occlusion queries use `GPUQuerySet` with `beginOcclusionQuery()`/`endOcclusionQuery()` on a render pass — [MDN beginOcclusionQuery](https://developer.mozilla.org/en-US/docs/Web/API/GPURenderPassEncoder/beginOcclusionQuery)
- Discover three.js: "Make your frustum as small as possible" (near/far tight) — [Discover three.js tips](https://discoverthreejs.com/tips-and-tricks/)

### Inferences
- Occlusion query results arrive a frame or more late, so naive query-based culling makes objects pop in when the camera turns a corner — a visible quality risk for smooth camera motion.
- Screen-size culling (hide objects whose projected sphere is < N px) is easy to add on top of a per-instance visibility API (BatchedMesh `setVisibleAt`, InstancedMesh2 visibility, or compacting InstancedMesh `count`); no sourced implementation found.
- Moving instances + stale bounds is the main culling *correctness* risk: recompute the InstancedMesh sphere after moves or disable culling on that mesh.

### Gaps
- No measured CPU cost of default frustum culling per object in current three.js.
- three-mesh-bvh is for per-geometry triangles; it does not do scene-level frustum culling.

## 5. Raycasting cost and three-mesh-bvh

### Takeaway
Default raycast is brute-force over all triangles of every candidate mesh (after a bounding-sphere test); on detailed CAD glTFs, hover-picking every pointermove is a classic frame-time spike. three-mesh-bvh makes per-geometry raycasts logarithmic and supports BatchedMesh; with instancing, one BVH per shared geometry serves all instances.

### Cited Findings
- three-mesh-bvh: "Casting 500 rays against an 80,000 polygon model at 60fps!"; patch `BufferGeometry.prototype.computeBoundsTree/disposeBoundsTree` and `Mesh.prototype.raycast = acceleratedRaycast`; `raycaster.firstHitOnly = true` uses the faster `raycastFirst` path; BatchedMesh supported via `computeBatchedBoundsTree`; `SkinnedMeshBVH` exists (recent API); standard BVH is not dynamic (no morph/skinning); off-thread build via `GenerateMeshBVHWorker`, parallel via `ParallelMeshBVHWorker` (needs SharedArrayBuffer) — [three-mesh-bvh README](https://github.com/gkjohnson/three-mesh-bvh)
- InstancedMesh raycast loops all instances, running Mesh.raycast per instance (so the geometry BVH is reused per instance) — [InstancedMesh.js source](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/objects/InstancedMesh.js)
- InstancedMesh2: per-instance raycasting via instance BVH; `raycastOnlyFrustum` limits to visible instances; recommends three-mesh-bvh for per-geometry BVH — [agargaro/instanced-mesh](https://github.com/agargaro/instanced-mesh)
- drcmda: raycasting traverses all suitable meshes; raycast only interactive objects; use three-mesh-bvh — [forum: performance of scene traversal](https://discourse.threejs.org/t/performance-of-scene-traversal/28537); manthrax lists avoiding per-frame raycasting as step 3 — [forum: when is LOD beneficial](https://discourse.threejs.org/t/when-is-it-actually-beneficial-to-use-lod-in-three-js-for-performance/87697)

### Inferences
- Cheap wins without new deps: raycast against a dedicated list of pickable roots (`raycaster.intersectObjects(pickables, true)`), throttle hover to one cast per animation frame (not per pointermove event), set `firstHitOnly`, and cast against simplified LOD/proxy geometry or bounding boxes for hover, reserving exact triangles for click.
- With InstancedMesh, 200 instances x BVH raycast is negligible; without BVH, 200 x (CAD part triangles) can be milliseconds.

### Gaps
- No direct benchmark of InstancedMesh + three-mesh-bvh vs per-clone BVH.

## 6. matrixAutoUpdate / scene graph update costs

### Takeaway
Deep-cloned glTF scenes create many Object3D nodes (each clone = full node hierarchy); every frame `scene.updateMatrixWorld()` recomposes each node's local matrix (if `matrixAutoUpdate`) and multiplies world matrices. Instancing collapses this to a few nodes. For remaining static nodes set `matrixAutoUpdate = false` and call `updateMatrix()` when they change.

### Cited Findings
- "Set `object.matrixAutoUpdate = false` for static or rarely moving objects" and call `updateMatrix()` manually — [Discover three.js tips](https://discoverthreejs.com/tips-and-tricks/)
- donmccurdy: ~6,700 meshes means "too many draw calls, and a large scene graph to traverse" — [forum: performance of scene traversal](https://discourse.threejs.org/t/performance-of-scene-traversal/28537)
- No measured per-node cost of updateMatrixWorld was found in that thread — [forum: performance of scene traversal](https://discourse.threejs.org/t/performance-of-scene-traversal/28537)

### Inferences
- At 200 clones x ~10 nodes = ~2,000 nodes, matrix updates are likely well under 1 ms on desktop but noticeable on low-end mobile; the draw calls cost more than the matrices. Moving an object then only needs its root's local matrix + world-matrix propagation to its children.
- `Object3D.matrixWorldAutoUpdate` (newer property) can stop world-matrix propagation for static subtrees; not verified in this pass.

### Gaps
- No benchmark numbers for scene-graph update cost vs node count in current three.js.

## 7. Benchmark numbers summary (all sourced above)
- gkjohnson cube demo (M1 Pro): InstancedMesh ~120 fps, merged ~120 fps, BatchedMesh ~30 fps at ~200k items — [#28776](https://github.com/mrdoob/three.js/issues/28776)
- Revit model ~100k sub-geometries: BatchedMesh ~20 fps vs Mesh ~60 fps — [#28776](https://github.com/mrdoob/three.js/issues/28776)
- sortObjects alone: 30 -> 9 fps in one test — [#28776](https://github.com/mrdoob/three.js/issues/28776)
- BatchedMesh 30-50% slower than merged at ~6,000 geometries — [#28776](https://github.com/mrdoob/three.js/issues/28776)
- multi-draw worse than instancing above ~100k instances (agargaro) — [forum](https://discourse.threejs.org/t/how-to-choose-between-instancedmesh-and-batchedmesh/81221)
- three-mesh-bvh: 500 rays vs 80k-poly model at 60 fps — [README](https://github.com/gkjohnson/three-mesh-bvh)
- LOD helps once triangles reach "several million per frame"; merging/instancing "10x-100x" (manthrax estimate) — [forum](https://discourse.threejs.org/t/when-is-it-actually-beneficial-to-use-lod-in-three-js-for-performance/87697)
- Gap: no published fps-vs-draw-call curve for 40-200 cloned glTFs; the kit should measure `renderer.info.render.calls`/`triangles` and frame time before and after.
