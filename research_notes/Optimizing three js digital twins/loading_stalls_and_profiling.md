# Loading stalls and profiling in three.js digital twins

Context: kit loads meshopt-compressed GLB with WebP textures via GLTFLoader + MeshoptDecoder/DRACOLoader/KTX2Loader, clones models per object, adds objects at runtime, and sees one long task of 100-1400 ms on a cold first load in Chrome. Research date: 2026-11-23.

## 1. Shader compilation: compile / compileAsync, program cache keys, variants

### Takeaway
`compileAsync` (r158+, PR #19752 merged Oct 2023) only helps with *program* compile/link, and only in a non-blocking way where KHR_parallel_shader_compile actually works (Chrome yes; Safari ships it but has reported blocking on Metal; Firefox no). It does not upload textures or geometry, it skips nothing that frustum culling hides, and it only helps if it is called under the *exact* conditions of the real render (same lights, environment, output color space / render target, tone mapping, composer passes). Program count is driven by the cache key, not by material count: clones with identical parameters share one program, but toggling `transparent`, `alphaTest`, `side`, `vertexColors`, maps, defines, light counts or fog creates a new program.

### Cited Findings
- `compile(scene, camera, targetScene)` "Compiles all materials in the scene with the camera ... useful to precompile shaders before the first rendering. If you want to add a 3D object to an existing scene, use the third optional parameter for applying the target scene." The (target) scene's lighting and environment must be configured before calling it. — [three.js docs, WebGLRenderer](https://threejs.org/docs/pages/WebGLRenderer.html)
- `compileAsync` "makes use of the KHR_parallel_shader_compile WebGL extension ... recommended to use this version of compile() whenever possible"; returns a Promise "that resolves when the given scene can be rendered without unnecessary stalling due to shader compilation." — [three.js docs](https://threejs.org/docs/pages/WebGLRenderer.html)
- PR #19752 "WebGLRenderer: Add compileAsync()" by toji, opened June 2020, merged October 14, 2023. With the extension, the code polls COMPLETION_STATUS before first use; systems without the extension "will immediately report that every program is ready, which removes some of the benefit." — [three.js PR #19752](https://github.com/mrdoob/three.js/pull/19752)
- KHR_parallel_shader_compile lets apps poll COMPLETION_STATUS_KHR "without potentially incurring stalls"; not Baseline. — [MDN](https://developer.mozilla.org/docs/Web/API/KHR_parallel_shader_compile)
- Browser support: Chrome since 76; Safari 14.1+ and iOS 14.5+ list it as supported; Firefox does not support it. — [caniuse](https://caniuse.com/wf-khr-parallel-shader-compile)
- WebKit bug 251514 (Feb 2023): "KHR_parallel_shader_compile does not allow for shader compilation without blocking" on Safari 16 / Apple Silicon; conformance test fails on Metal (bug 243820 duped to it). Fix status not found. — [WebKit bug 251514](https://lists.webkit.org/pipermail/webkit-unassigned/2023-February/1084921.html)
- Forum report (Aug 2025): `compileAsync` on InstancedMeshes had no visible effect; first-seconds stutter disappeared only after a one-off render with every mesh forced `visible = true` and `frustumCulled = false`, then restored. Participants argued much of the "shader stutter" is really first texture/geometry upload, which happens on first real draw, not in compileAsync. (Community inference, no maintainer confirmation.) — [three.js forum #86191](https://discourse.threejs.org/t/reduce-first-render-lag-renderer-compileasync-is-not-doing-anything/86191)
- Forum write-up (Sept 2026, GroDev): program compile happens on the first frame; after each compile three.js queries the driver (`getProgramParameter`, uniform locations) and "each query waits for the GPU's command queue to drain." A production scene with 14 programs (EffectComposer + UnrealBloomPass) produced one 656 ms task; after warming, first frame dropped to "a few ms" and TBT from ~700 ms to 140 ms. The render target's color space is part of the program key, so materials compiled for the screen recompile when drawn into a composer buffer; `Points` use a different pipeline, so warm with a real `render()`. Suggested: fence (`gl.fenceSync` + non-blocking `clientWaitSync` loop) after compileAsync when the extension is missing; warm one program per frame to a 1x1 target. (Post also promotes a paid product; numbers are self-reported.) — [three.js forum #94734](https://discourse.threejs.org/t/why-a-heavy-three-js-scene-tanks-pagespeed-and-it-is-not-your-javascript/94734)
- Same theme: "getProgramParameter freezes app" report; precompiling via compileAsync suggested; noted not async on Firefox. — [three.js forum #88368](https://discourse.threejs.org/t/webglrenderer-getprogramparameter-freezes-app/88368)
- Open question (Jan 2026): `WebGLPrograms` computes the key from the renderer's current `outputColorSpace`; compiling for a cube camera / render target while the main loop renders in sRGB causes recompiles. compileAsync calls sync `compile` internally then waits, so keys are fixed at call time. No maintainer answer. — [three.js forum #89007](https://discourse.threejs.org/t/webglrenderer-compile-async-and-possible-wrong-shader-cache-key/89007)
- Program cache key (current `dev` WebGLPrograms.js) contains: shaderID (or custom vertex/fragment shader IDs), every `defines` entry, precision, outputColorSpace, envMap mode/size, fog type, morph count, shadowMapType, toneMapping, clipping plane counts, the UV channel of every active map (presence of each map), all light counts by type and shadow counts, bitmasks for instancing, instancingColor, batching, vertexColors, vertexAlphas, vertexTangents, alphaTest, alphaHash, skinning, morph targets, fog, flatShading, logarithmicDepthBuffer, premultipliedAlpha, shadowMapEnabled, doubleSided, flipSided, dithering, transmission, sheen, opaque, alphaToCoverage, plus `material.customProgramCacheKey()`. Not in the key: material uuid, name, uniform values (color, roughness, opacity, light positions). Programs are looked up by key and reference-counted (`usedTimes`), destroyed when the count hits zero. — [three.js WebGLPrograms.js](https://raw.githubusercontent.com/mrdoob/three.js/dev/src/renderers/webgl/WebGLPrograms.js)
- `customProgramCacheKey` is needed when `onBeforeCompile` patches shaders differently per material; without distinct keys materials collide; the callback is not carried by `.clone()/.copy()/.toJSON()`; random per-material keys create a new program every time. — [forum: defines vs customProgramCacheKey](https://discourse.threejs.org/t/defines-vs-customprogramcachekey/43711); [forum #38614](https://discourse.threejs.org/t/onbeforecompile-fires-only-once-although-i-set-needsupdate-true/38614); [forum #53340](https://discourse.threejs.org/t/material-material-onbeforecompile-problem-for-three-js/53340)
- GLTFLoader caches one material instance per glTF material index, but `assignFinalMaterial` clones a material when a primitive lacks tangents/normals or has vertex colors (cached under a `ClonedMaterial:` key) — those clones differ in key flags (vertexColors, flatShading) and so can be separate programs. — [GLTFLoader.js](https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/jsm/loaders/GLTFLoader.js)

### Inferences
- For the kit: cloning a model per object with `object.clone()` shares materials (shallow), and even deep-cloned materials with identical settings reuse a program, so clones are not the program multiplier. The multipliers are state toggles: making a selected/ghosted object `transparent` (flips `opaque`), changing `side`, adding/removing a map, changing light/shadow counts at runtime (every lit program recompiles when a light is added), `onBeforeCompile` with ad-hoc keys, and render-target color space (picking/outline/composer passes).
- Practical recipe: (1) fix the lighting rig, environment, shadow config and tone mapping before load; (2) for each new model type, `await renderer.compileAsync(model, camera, scene)` before `scene.add`; (3) also pre-compile the "highlight/ghost/x-ray" variant materials once at startup (e.g. render a hidden 1x1 test mesh with each); (4) do one warm render with frustumCulled off (or call `initTexture` on each texture) so uploads happen before reveal; (5) track `renderer.info.programs.length` as a regression metric.
- On Chrome/ANGLE-Metal (Mac), compileAsync should move compile/link off the critical path; on Safari, expect less help per bug 251514; on Firefox none. A 100-1400 ms single task is consistent with synchronous compile + the follow-up `getProgramParameter`/`getUniformLocation` sync waits plus first texture uploads happening in the first `render()`.

### Gaps
- No measured numbers found for compileAsync savings specifically on Chrome ANGLE-Metal vs Safari in 2025-2026; whether WebKit fixed bug 251514 is unknown.
- No official three.js statement on whether compileAsync honours `frustumCulled`/`visible` (forum users suspect objects not visible are skipped; not verified in source).

## 2. Texture upload stalls: initTexture, ImageBitmapLoader, KTX2 vs WebP

### Takeaway
WebP/PNG/JPEG shrink the download but are decoded to full RGBA on the CPU and uploaded uncompressed (width x height x 4 x 1.33 with mips — ~90 MB for one 4K texture); the upload (`texImage2D`) on the main thread is often the stall. KTX2/Basis stays block-compressed on the GPU (4-8x less memory, 4-8x faster upload), transcoding runs in workers. Use `renderer.initTexture()` to upload ahead of first draw, and spread uploads across frames.

### Cited Findings
- `initTexture(texture)`: "Initializes the given texture. Useful for preloading a texture rather than waiting until first render (which can cause noticeable lags due to decode and GPU upload overhead)." — [three.js docs](https://threejs.org/docs/pages/WebGLRenderer.html)
- Memory formula: width x height x 4 x 1.333 (RGBA, mipmaps); a 4096x4096 texture is ~90 MB in memory regardless of file size. PNG/JPEG/WebP/AVIF are decompressed on CPU and uploaded uncompressed. GPU formats cut memory 4-8x and "finish uploads 4-8x faster"; `initTexture` uploads earlier and `ImageBitmapLoader` moves decompression off the main thread, "which reduces dropped frames somewhat." ETC1S ~JPEG-size files, weak for normal maps; UASTC (+Zstd) 1-2x larger than JPEG, good for normals/data, still ~4x smaller in memory than PNG/JPEG. Recommendation: WebP/AVIF if first paint is top priority; KTX2/Basis for long sessions or heavy interactive texture loading. (Feb 2024) — [Don McCurdy, Choosing texture formats](https://www.donmccurdy.com/2024/02/11/web-texture-formats/)
- Forum example: 4096x4096 texture ~1 MB as PNG but ~88 MB in VRAM. — [three.js forum #86191](https://discourse.threejs.org/t/reduce-first-render-lag-renderer-compileasync-is-not-doing-anything/86191)
- GLTFLoader uses `ImageBitmapLoader` (createImageBitmap) except when createImageBitmap is undefined, Safari < 17, or Firefox < 98, where it falls back to `TextureLoader`. Identical texture requests share one promise (`textureCache`); repeated image sources return `texture.clone()` (`sourceCache`). — [GLTFLoader.js](https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/jsm/loaders/GLTFLoader.js)
- Chrome is the only browser found to decode `createImageBitmap` images concurrently; Firefox decodes them serially on the main thread (Sponza: Chrome 177 ms createImageBitmap vs Firefox 460 ms; Firefox ImageDecoder 116-196 ms). Recommendation: Chrome createImageBitmap, Safari HTMLImageElement, Firefox ImageDecoder. (May 2026) — [Ignacio Castaño, Image Loading on the Web](https://www.ludicon.com/castano/blog/2026/05/image-loading-on-the-web/)
- Chrome profiler shows time in `texImage2D`; uploading an ImageBitmap in Chrome took 165 ms for an 8192x4096 JPEG vs 500 ms for an Image element (older report); Chrome's createImageBitmap decodes off-thread only for Blob sources. WebKit: each texImage2D copies the ImageBitmap multiple times and sends it over IPC. — [three.js forum #44579](https://discourse.threejs.org/t/loader-textureloader-imagebitmaploader-blocks-js-main-thread-causing-temporary-freeze/44579); [Lightrun/three.js issue](https://lightrun.com/answers/mrdoob-three-js-use-imagebitmaps-for-gltfloader-and-possibly-others); [WebKit bug 234920](https://bugs.webkit.org/show_bug.cgi?id=234920)
- KTX2Loader: `setWorkerLimit(n)` caps transcoder Web Workers; `detectSupport(renderer)` must be called before loading (`detectSupportAsync` deprecated); WASM transcoder in `examples/jsm/libs/basis`. — [three.js docs, KTX2Loader](https://threejs.org/docs/pages/KTX2Loader.html)
- gltf-transform `optimize` defaults: `--texture-compress auto`, `--texture-size 2048`; `ktx2` mode runs UASTC then ETC1S passes via `toktx`. — [gltf-transform cli.ts](https://raw.githubusercontent.com/donmccurdy/glTF-Transform/main/packages/cli/src/cli.ts)

### Inferences
- Budget per texture (uncompressed, with mips): 1024² ≈ 5.6 MB, 2048² ≈ 22 MB, 4096² ≈ 89 MB (from the formula above). A twin with 20 distinct 2K WebP textures ≈ 450 MB VRAM; same in ETC1S/UASTC ≈ 55-110 MB.
- Mipmap generation for non-compressed textures (`generateMipmap`) runs as part of upload in three.js; KTX2 files can carry pre-built mips, so no runtime generation. (No measured cost found — see gaps.)
- For per-object clones, textures are shared via glTF caches and `Object3D.clone()`, so cost is per unique texture, not per clone; but loading the same GLB twice through separate `loader.load` calls creates separate textures — cache the parsed gltf per URL.
- Suggested: convert large/most-reused textures (≥1K, base color/ORM/normal) to KTX2 (ETC1S for color, UASTC for normals), keep tiny ones WebP; cap at 1K-2K; after load, call `renderer.initTexture(t)` for each new texture one or a few per frame before `scene.add`.

### Gaps
- No current measurements of `generateMipmap` cost or of KTX2 transcode ms per texture in three.js were found.
- Whether Chrome's 2025-2026 GPU process makes `texImage2D(ImageBitmap)` fully async was not confirmed.

## 3. Geometry decode and gltf-transform optimize options

### Takeaway
Meshopt decode is very fast (1-3 GB/s with WASM SIMD; ~10-100x faster than Draco in zeux's benchmark) and can run in workers via `decodeGltfBufferAsync` + `useWorkers`; Draco gives smaller files but decodes slowly (DRACOLoader uses workers). Geometry decode is unlikely to be the 100-1400 ms task with meshopt. `optimize` defaults instance/palette/flatten/join/weld/simplify to true; the kit's `--instance false --flatten false` keeps per-node structure (needed for per-part selection) but forgoes join (join requires flatten) and GPU instancing.

### Cited Findings
- "When SIMD is available, the decoders run at 1-3 GB/s on modern desktop computers"; scalar and SIMD implementations auto-selected; `useWorkers(n)` once at startup; `decodeGltfBufferAsync` returns a Promise (for large buffers). — [meshoptimizer js/README](https://raw.githubusercontent.com/zeux/meshoptimizer/master/js/README.md)
- Benchmark (zeux, i7-8650U, ~2019): Sponza 184k verts — Draco ~19 ms vs meshopt ~1.9 ms; Buddha/Bistro — Draco ~400-780 ms vs meshopt ~7-8 ms; Draco files smaller. — [zeux gist](https://gist.github.com/zeux/b48678a44944187c16f55ad6cca45e78)
- GLTFLoader calls `meshoptDecoder.decodeGltfBufferAsync` when present (decoder ≥0.19), else sync `decodeGltfBuffer`. — [GLTFLoader.js](https://raw.githubusercontent.com/mrdoob/three.js/dev/examples/jsm/loaders/GLTFLoader.js)
- `optimize` option defaults and order: dedup → instance (`--instance true`, `--instance-min`) → palette (`--palette true`: "create palette textures and merge materials") → flatten (`--flatten true`) → join (`--join true`, "requires flatten"; `--join-meshes`, `--join-named`) → weld (`--weld true`, "required for simplification") → simplify (`--simplify true`, `--simplify-error`, `--simplify-ratio`, `--simplify-lock-border`) → resample → prune (`--prune-attributes`, `--prune-solid-textures` converts solid textures to factors) → sparse → texture compress (`auto`, max `--texture-size 2048`) → mesh compress (`--compress meshopt` default, `--meshopt-level high`). — [gltf-transform cli.ts](https://raw.githubusercontent.com/donmccurdy/glTF-Transform/main/packages/cli/src/cli.ts); [gltf-transform CLI docs](https://gltf-transform.dev/cli)
- Command meanings: instance "creates GPU instances from shared mesh references"; flatten "flattens the scene graph"; join "joins meshes to reduce draw calls"; palette "creates palette textures and merges materials"; simplify "reduces mesh vertex count"; weld "merges equivalent vertices"; dedup dedups accessors/textures; prune removes unreferenced properties. — [gltf-transform CLI docs](https://gltf-transform.dev/cli)
- gltfpack equivalents: `-mi` instancing (EXT_mesh_gpu_instancing, keeps `_COLOR_0` for tinting); meshes on non-animated nodes merged by default; `-km` keep named materials; `-kn` keep named nodes for external transforms; `-tc` KTX2/Basis; `-tw` WebP; `-si R` simplify ratio. gltfpack does not support Draco. — [meshoptimizer.org/gltf](https://meshoptimizer.org/gltf/)

### Inferences
- `--flatten false` disables join (join requires flatten), so each part stays its own draw call. A selective alternative for twins: keep named nodes (`--join-named false`, gltfpack `-kn`) so addressable parts survive, while letting unnamed static sub-meshes join. Palette merges many flat-color materials into one material + small palette texture, cutting both draw calls (when combined with join) and programs.
- `--instance false` is reasonable when the kit itself instances per-object clones at runtime (InstancedMesh/BatchedMesh); in-file instancing only helps repeated meshes inside one GLB.
- Simplify is lossy; for CAD-derived machinery use a small `--simplify-error` and lock borders, or generate LODs instead.
- Draco is only worth keeping for very large download-bound meshes; prefer meshopt for runtime-added objects.

### Gaps
- No 2024-2026 head-to-head decode benchmark in browsers was found; zeux's numbers are ~2019 hardware. DRACOLoader default worker count (believed 4 via `setWorkerLimit`) not verified in this session.
- Exact numeric defaults of `INSTANCE_DEFAULTS.min`, `PALETTE_DEFAULTS.min`, `SIMPLIFY_DEFAULTS` were not visible in the CLI source fetched.

## 4. Spreading work across frames and pre-warming

### Takeaway
Treat "object appears" as a pipeline: fetch/parse (async, workers) → compileAsync for new program keys → initTexture/geometry upload a slice per frame → add to scene. Yield between steps with `scheduler.yield()` (Chrome 129+, Firefox 142+, not Safari) or a setTimeout fallback, and keep each slice well under one frame (~4-8 ms at 60 fps).

### Cited Findings
- `scheduler.yield()` pauses and resumes with higher priority than new tasks (unlike setTimeout); requestIdleCallback / `postTask({priority:'background'})` work waits until most other tasks finish; support: Chrome 129, Edge 129, Firefox 142, Safari not supported; feature-detect and fall back to a setTimeout promise, yielding about every 50 ms. — [web.dev Optimize long tasks](https://web.dev/articles/optimize-long-tasks); [Chrome: Use scheduler.yield()](https://developer.chrome.com/blog/use-scheduler-yield); [Chrome: Break up long tasks](https://developer.chrome.com/docs/modern-web-guidance/guides/performance/break-up-long-tasks)
- Warm each program with one draw to a 1x1 target, one per frame with a fence between; split library/renderer/scene construction with yields; don't do heavy work in a single task at `load`. — [three.js forum #94734](https://discourse.threejs.org/t/why-a-heavy-three-js-scene-tanks-pagespeed-and-it-is-not-your-javascript/94734)
- Force a one-time render of all meshes (visible, frustumCulled=false) before showing the scene removed first-look stutter. — [three.js forum #86191](https://discourse.threejs.org/t/reduce-first-render-lag-renderer-compileasync-is-not-doing-anything/86191)
- Creating ImageBitmaps in a worker and transferring them works well, though in Chrome it uses several background threads that can compete with rendering. — [WebGL Fundamentals Q&A](https://webglfundamentals.org/webgl/lessons/webgl-qna-how-to-load-images-in-the-background-with-no-jank.html)

### Inferences
- For a palette drop or data-driven batch of N objects: group by model URL; for each new URL run the pipeline once; then clones are cheap (no new program/texture/geometry if materials/geometries are shared), so only the first object of a type should cost anything. Add clones in batches per frame and switch to InstancedMesh/BatchedMesh when counts are high.
- requestIdleCallback is poorly suited to a continuously rendering rAF loop (idle periods may be rare); a frame-budgeted queue processed at the start/end of each rAF tick is more predictable.
- GL work in rAF is attributed by LoAF/long-task to the rAF callback, so sliced work shows as shorter frames rather than one task.

### Gaps
- No source quantified per-frame slice size for uploads/compiles in three.js; budget above is an inference from 16.7 ms frame time.

## 5. Measuring: renderer.info, stats-gl, Spector, DevTools, LoAF, CI

### Takeaway
Use `renderer.info` for counts (calls, triangles, programs, geometries, textures), stats-gl for CPU+GPU frame time (timer queries; Safari needs a feature flag), LoAF (Chrome 123+) via PerformanceObserver to catch hitches in the field and in CI, and a cold profile (fresh profile, shader cache cleared) because repeat loads hide compile cost.

### Cited Findings
- `renderer.info`: `memory.geometries`, `memory.textures`, `render.frame`, `render.calls`, `render.triangles`, `render.points`, `render.lines`, `programs` (array of WebGLProgram); reset every render unless `info.autoReset = false` and `info.reset()` per frame (needed with multiple passes/post-processing). — [three.js docs](https://threejs.org/docs/pages/WebGLRenderer.html)
- stats-gl: `new Stats({ trackGPU: true })`, `stats.init(renderer)` (WebGL or WebGPU), `stats.update()` per frame; options trackFPS (true), trackGPU (false, "requires extension support"), trackHz, trackCPT (WebGPU compute), logsPerSecond 4, samplesLog 40; WebGPU uses timestamp-query; Safari needs WebKit "WebGL Timer Queries" feature flag. — [stats-gl README](https://github.com/RenaudRohlinger/stats-gl)
- LoAF: a frame whose rendering update is delayed > 50 ms; fields `duration`, `blockingDuration`, `renderStart`, `styleAndLayoutStart`, `firstUIEventTimestamp`, `scripts[]` (>5 ms, with `invoker`, `sourceURL`, `sourceFunctionName`, `forcedStyleAndLayoutDuration`); shipped Chrome 123 (Edge 123), not Firefox/Safari; observe with `PerformanceObserver({type:'long-animation-frame', buffered:true})` (buffer holds 200 entries); captures many short tasks that add up, unlike Long Tasks (Chrome 58+). The article does not address GPU/WebGL attribution. — [Chrome LoAF docs](https://developer.chrome.com/docs/web-platform/long-animation-frames)
- Profiling pitfalls: desktop DevTools on repeat loads misleads because cached shader programs remove compilation — use a fresh profile (author also used `--disable-gpu` to mimic Lighthouse SwiftShader); headless Lighthouse/PageSpeed renders WebGL in software; long tasks get labelled by whichever library registered rAF first; "Other" in the main-thread breakdown includes GPU waits; read long tasks by start time; PSI noise 7-10 points, use median of three. — [three.js forum #94734](https://discourse.threejs.org/t/why-a-heavy-three-js-scene-tanks-pagespeed-and-it-is-not-your-javascript/94734)

### Inferences
- CI recipe: Playwright + Chrome (with real GPU or a fixed SwiftShader baseline), fresh user-data-dir each run, load the twin, add N objects from the palette, and collect (a) LoAF entries' max `duration`/sum `blockingDuration`, (b) rAF frame deltas (p95, p99, max), (c) `renderer.info.programs.length`, `memory.textures`, `memory.geometries`, `render.calls`. Fail on thresholds (e.g. no LoAF > 100 ms after first paint, p99 frame < 33 ms, programs ≤ baseline). Software GL timings are only useful relative to a baseline, not absolute.
- Programs-count is the single best leading indicator of first-show hitches; texture count × size the best indicator of upload stalls and VRAM.
- Spector.js (frame capture of GL calls/state) and the Chrome Performance panel GPU track are the tools to attribute a hitch to compile vs upload.

### Gaps
- Spector.js, r3f-perf, and Chrome DevTools GPU-track documentation were not fetched in this session (tool budget); their details are not cited here.
- EXT_disjoint_timer_query_webgl2 availability in Chrome (often disabled on some platforms for security) was not verified.

## 6. Typical budgets for smooth web 3D

### Takeaway
No authoritative standard exists; community rules of thumb are ~100 (≤ 100-200) draw calls on low-end mobile, a few hundred on laptops, 50k-100k triangles per frame for mobile/VR up to ~1M on desktop, and texture memory kept to a few hundred MB on desktop and far less on phones. Draw calls usually matter more than triangles.

### Cited Findings
- Mobile: 100-200 draw calls as a rough low-end target; another source: < 100 for 60 fps, 100-500 a warning zone; < 1M triangles at 60 fps; VR rule of thumb 50k-100k triangles per frame (called a guess by a forum user); case study: merging cut draw calls 387 → 42 and mobile fps 10 → ~30. These are rough community guidelines. — [three.js forum: performance on mobile](https://discourse.threejs.org/t/problems-with-performance-on-mobile-devices/23734); [PlayCanvas optimization guidelines](https://developer.playcanvas.com/user-manual/optimization/guidelines/); [threejs-journey performance tips](https://www.threejs-journey.com/lessons/performance-tips); [three.js forum: scene optimization limits](https://discourse.threejs.org/t/three-scene-optimization-texture-size-calculation-in-gpu-compression-merge-meshes-what-the-limits/23972)
- Particles/vertices are cheap; fill cost (bloom, transparency, overdraw) is often the real limit. — [three.js forum #94734](https://discourse.threejs.org/t/why-a-heavy-three-js-scene-tanks-pagespeed-and-it-is-not-your-javascript/94734)

### Inferences
- For a twin kit: target ≤ 100 draw calls on phones / ≤ 300-500 on laptops, ≤ 300k-500k visible triangles on phones / ~1-2M on laptops, ≤ 256 MB textures on laptops and ≤ ~100 MB on phones (KTX2 makes this attainable), and ≤ ~20-30 programs total. These are synthesized targets, not sourced standards.

### Gaps
- No vendor-published (Apple/Google/Khronos) budgets for WebGL draw calls or VRAM were found; numbers above come from community/engine guidance and vary widely by device.
