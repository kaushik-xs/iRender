# Steady frame times in three.js digital twins: shadows, render loop, HTML labels, workers, WebGPU, pixel ratio

Context: one DirectionalLight with a 2048 PCF shadow map covering the whole plant, a RoomEnvironment PMREM, NeutralToneMapping, rendering on demand but continuously while anything moves, and 20-60 DOM tags placed by projection plus a collision pass that reads cached sizes. Research date: 2026-11-23. Current three.js is about r183-r184.

## Shadows: per-frame cost, static caching, map type/size, baking, fake shadows

### Takeaway
A shadow-casting DirectionalLight adds a second depth pass over every caster on every frame it updates. The main lever is to stop updating it: `light.shadow.autoUpdate = false` per light, or `renderer.shadowMap.autoUpdate = false`, then set `needsUpdate = true` only when a caster or the light moves. As of r182, `PCFSoftShadowMap` is deprecated on WebGLRenderer, and plain `PCFShadowMap` is now soft, so the kit's current choice is right. Moving objects can use cheap fake shadows (blob decal or ContactShadows with a limited number of frames) instead of forcing the big map to re-render.

### Cited Findings
- Pattern: set `renderer.shadowMap.autoUpdate = false`, and set `renderer.shadowMap.needsUpdate = true` when an update is required. The answer also says baked shadows "will definitely give you the best performance" for a fully static scene. — [three.js forum: How to render shadow map one time](https://discourse.threejs.org/t/how-to-render-shadow-map-one-time/7176)
- Cost when misused: one user ran at 60 fps with static shadows but dropped to about 20 fps after setting needsUpdate on every change. In the same thread, draw calls roughly halved when autoUpdate was disabled, which shows the shadow pass's share of draw calls. — [three.js forum: renderer.shadowMap.autoUpdate == false](https://discourse.threejs.org/t/renderer-shadowmap-autoupdate-false/50401)
- Version caveat: the old hack of setting `renderer.shadowMap.enabled = false` after the first render to keep a cached map stopped working in r165/r166. The suggested replacement is `light.shadow.autoUpdate = false` (with `light.shadow.needsUpdate`), which also lets you choose per light which shadow stays dynamic. The thread has no maintainer confirmation that this was a bug. — [three.js forum: shadowMap can't be cached after r164](https://discourse.threejs.org/t/when-three-js-version-bigger-than-164-the-shadowmap-cant-be-cache-this-leads-to-a-decrease-in-rendering-performance/67687)
- Resize gotcha: changing the shadow map size while autoUpdate is off needed `shadow.map.setSize()` (or disposing the map) before it rendered correctly. — [three.js forum: ShadowMap not resizing correctly](https://discourse.threejs.org/t/shadowmap-not-resizing-correctly-when-scene-autoupdate-false/34474)
- Official migration note, r181 to r182: "`PCFSoftShadowMap` with `WebGLRenderer` is now deprecated. Use `PCFShadowMap` which is now soft as well." — [three.js Migration Guide](https://github.com/mrdoob/three.js/wiki/Migration-Guide)
- r182 to r183: "The shadows in `WebGPURenderer` have been improved. It might be necessary to remove or decrease shadow bias values." — [three.js Migration Guide](https://github.com/mrdoob/three.js/wiki/Migration-Guide)
- Regression reports after the r182 shadow rewrite: extreme pixelation in 0.182 compared with 0.181 using identical code, and point light shadows broken until bias was changed. Separately, a report describes artifacts with WebGPURenderer plus VSMShadowMap. — [Shadow pixelation in 0.182.0](https://discourse.threejs.org/t/shadow-pixelation-issue-in-three-js-0-182-0/88746); [Upgrading to r182: point light shadows](https://discourse.threejs.org/t/upgrading-to-r182-incorrect-point-light-shadows/88891); [Shadow artifacts with WebGPURenderer VSMShadowMap](https://discourse.threejs.org/t/shadow-artifacts-with-webgpurenderer-vsmshadowmap/89017)
- A WebGPU user reports a performance drop and lower shadow quality in r182 compared with WebGL r170. — [three.js forum](https://discourse.threejs.org/t/webgpu-significant-performance-drop-and-shadow-quality-regression-in-r182-vs-webgl-r170/89322)
- drei ContactShadows is "a rather expensive effect". For static objects, `frames={1}` renders it only once. The example uses `resolution={256}` plus `blur` and `far`. — [drei docs: ContactShadows](https://drei.docs.pmnd.rs/staging/contact-shadows)
- Forum consensus for a scene with many lights where nearly everything is static: bake the static shadows and keep runtime shadows for the few dynamic things. — [Many lights, nearly everything static: Shadows?](https://discourse.threejs.org/t/many-lights-nearly-everything-static-shadows/9803); [How to optimize shadow rendering](https://discourse.threejs.org/t/how-to-optimize-shadow-rendering-in-three-js-for-better-performance/64681)
- pmndrs ships a `BakeShadows` helper that sets shadowMap.autoUpdate to false, which shows the ecosystem treats static shadow caching as the default optimisation. — [drei docs (bake-shadows.mdx mirror)](https://cdn.jsdelivr.net/npm/glodrei@0.0.2/docs/performances/bake-shadows.mdx)

### Inferences
- For the kit, one 2048 map over the whole plant means each update re-draws every caster (the plant's full mesh set) into a 4M-texel depth target. With objects moving on paths, `autoUpdate` would be true almost all the time. Two good splits:
  - (a) A static-caster pass baked once, with `light.shadow.autoUpdate = false` and `needsUpdate` only on layout or edit. Moving objects get blob or ContactShadow-style fake shadows.
  - (b) Two lights: a static cached light covering the plant and a small, tightly fitted second shadow light following the camera or the moving objects. Option (b) doubles the shading cost of the lit materials, so (a) is usually cheaper.
- Throttling `needsUpdate` (for example every 2nd-4th frame for moving casters) is a middle ground. It trades a slight shadow lag for about half to a quarter of the shadow-pass cost. I found no benchmark, so this is my inference.
- Fitting the shadow camera tightly to the plant's bounding box (orthographic left/right/top/bottom/near/far) gives better texel density per 2048 map than raising the size to 4096, which costs 4x the memory and fill. This is standard three.js practice from background knowledge; I did not fetch a source in this session.
- VSM gives softer penumbrae but needs blur passes and is prone to light bleeding. Given the recent VSM/WebGPU artifact reports, PCF (soft since r182) is the safer default.
- Baked AO or lightmaps (Blender bake, then a glTF lightMap/aoMap via uv2) remove the runtime cost entirely for static equipment. The trade-off is pipeline complexity and no response to layout edits. For a configurable twin whose layout changes, a cached runtime shadow map that re-renders on edit is the more practical "bake".

### Gaps
- I found no measured numbers (ms per frame) for a 2048 PCF directional shadow pass at plant scale. Cost scales with caster draw calls and triangles, so it should be profiled in the kit itself.
- I did not fetch the AccumulativeShadows docs. They are known to accumulate many jittered shadow frames into a texture, which suits static scenes, but I have no citable cost figure.
- I did not verify gltf-transform lightmap or AO baking support. gltf-transform does not bake lighting; baking happens in Blender. That is background knowledge and unverified here.

## Render loop: on-demand rendering, damping, frame pacing, long tasks, GC

### Takeaway
Rendering on demand is the established pattern. Controls with damping must keep requesting frames until the damping settles, and repeated invalidations must coalesce into one frame. Main-thread work outside rendering (data ingestion, layout, collision) must be kept under 50 ms per task and ideally a few ms per frame, by yielding (`scheduler.yield`, which Safari lacks) and by reusing scratch objects to avoid GC.

### Cited Findings
- r3f's `frameloop="demand"` renders only when something changes. Changes React can't see (such as camera controls) must call `invalidate()`. "Calling invalidate multiple times will not render multiple times", so requests coalesce into one frame. drei controls invalidate automatically. — [r3f docs: Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- Adaptive quality: r3f's `regress()` plus `performance.current` (1 means full quality) lets the app lower resolution, switch off lights or skip post-processing while the user interacts, then restore after a debounce (example uses 200 ms). The example sets `setDpr(window.devicePixelRatio * current)`. — [r3f docs: Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- PerformanceMonitor averages FPS and fires `onIncline` and `onDecline` with a gap between bounds to avoid ping-ponging. `flipflops` limits oscillation and then fires `onFallback`. — [r3f docs: Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- Draw-call guidance: "no more than 1000 as the very maximum", aim for "a few hundred or less". Use InstancedMesh, share geometries and materials, and reuse one temp Object3D instead of allocating per instance. — [r3f docs: Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- "Any task that takes longer than 50 milliseconds is a long task." `scheduler.yield()` yields to the main thread with a prioritised continuation. It is supported in Chrome/Edge 129 and Firefox 142, **not Safari**, so a fallback is needed. Do not yield after every call: batch small jobs and yield only when enough time has passed since the last yield. — [web.dev: Optimize long tasks](https://web.dev/articles/optimize-long-tasks)

### Inferences
- For the kit's "on demand but continuous while moving" loop, the robust pattern is:
  - Keep one `needsFrame` flag. Set it from the controls' `change` event, path animation and data updates.
  - In the rAF or `setAnimationLoop` callback, call `controls.update()`. With damping enabled it keeps emitting `change` until the velocity decays. Then render, and re-request only if the flag was set again.
  - This is the three.js manual's "rendering on demand" pattern. The manual page returned 404 at its old URL, so I could not quote it.
- At a 60 Hz display the frame budget is 16.7 ms, and at 120 Hz (ProMotion) it is 8.3 ms. For smoothness, all main-thread JS plus render submission should stay well under that, leaving headroom for compositing and the browser.
- rAF vs `renderer.setAnimationLoop`: setAnimationLoop wraps rAF and is required for WebXR and for WebGPURenderer (which needs async init before the first render), so it is the forward-compatible choice. There is no performance difference for non-XR use. Background knowledge; not fetched this session.
- GC: allocating `new Vector3()` for each of 60 labels per frame creates about 3,600 short-lived objects per second at 60 fps. That is minor on V8's young generation but avoidable. Hoist module-level scratch vectors, as r3f recommends with its reused temp Object3D.
- Time-slice heavy work that is not per-frame (parsing telemetry, rebuilding the BVH, collision when many tags change) using `scheduler.postTask` / `scheduler.yield` where available, with a `setTimeout(0)` or MessageChannel fallback for Safari. Use `requestIdleCallback` only for truly deferrable work. Safari shipped rIC late, so verify it.

### Gaps
- The three.js manual "Rendering on demand" page returned 404 at the URLs tried (manual moved to /manual/#en/... and the raw GitHub path changed). I have no direct quote on OrbitControls damping.
- Safari support status of `requestIdleCallback` and `scheduler.postTask` was not verified in this session.

## HTML labels: CSS2DRenderer / drei Html vs manual projection, layout thrash, WebGL text

### Takeaway
DOM labels are fine at tens, not hundreds. Anecdotal reports show Chrome falling off around 50+ drei `<Html>` elements, while 100+ troika SDF labels ran at 140 fps on old hardware. The kit's approach (manual projection, transform-only writes, cached sizes, no reads in the loop) is the cheap end of DOM labelling. Further wins come from culling (off-screen, behind camera, occluded), skipping work when the camera and objects did not move, and switching to WebGL text (troika) if counts grow or depth occlusion is needed.

### Cited Findings
- drei Html with more than about 50 elements dropped to below 5 fps in Chrome, with no issue in Firefox (anecdotal; drei Html, not CSS2DRenderer). — [Many HTML elements from drei causes low framerate in chrome](https://discourse.threejs.org/t/many-html-elements-from-three-js-fiber-drei-causes-low-framerate-in-chrome/28540)
- A user saw CSS2DRenderer taking over 80% of the frame profile even at idle, with matrixAutoUpdate disabled. CSS2DRenderer walks the whole scene and writes styles every render call. — [Does CSS2DRenderer ignore autoMatrixUpdate = false?](https://discourse.threejs.org/t/does-css2drenderer-ignore-automatrixupdate-false/67064)
- For large counts, forum advice is to pool labels (for example 100 troika labels plus points for the rest) rather than create thousands. — [Performant approach for displaying text labels (10000)](https://discourse.threejs.org/t/performant-approach-for-displaying-text-labels-10000/21863); [How to create lots of optimized 2D text labels](https://discourse.threejs.org/t/how-to-create-lots-of-optimized-2d-text-labels/66927)
- More than 100 dynamic troika-three-text labels updating every second ran at over 140 fps on older hardware (user report). — [100+ dynamic Troika labels with 1s updates](https://discourse.threejs.org/t/100-dynamic-troika-three-text-labels-with-1s-updates/90287)
- drei Html modes:
  - `occlude` hides labels behind geometry by setting opacity. `occlude="blending"` makes the HTML depth-correct as part of the 3D scene and is the heavier mode.
  - `transform` applies matrix3d, and text "may look blurry on some devices".
  - `zIndexRange` and `distanceFactor` control stacking and scaling.
  — [drei docs: Html](https://drei.docs.pmnd.rs/misc/html)
- troika-three-text:
  - Parses fonts and generates glyph SDFs on the fly in a web worker "to prevent frame drops". It patches any material, so lighting and fog apply.
  - `sdfGlyphSize` defaults to 64; larger values sharpen corners and thin strokes but cost memory and time.
  - Supports outlines and soft outlines.
  - The first use of a font can pause noticeably, which `preloadFont` mitigates. Layout is async, so bounds are not ready immediately. Removed Text must be disposed.
  — [troika-three-text docs](https://protectwise.github.io/troika/troika-three-text/)

### Inferences
- The kit already avoids the biggest cost, which is layout thrash from interleaving DOM reads (offsetWidth, getBoundingClientRect) with style writes in the loop. Keep sizes cached and re-measured only on content change, for example via ResizeObserver, outside the frame.
- Write only `transform: translate3d(x,y,0)` and opacity, because these are compositor-only properties. Add `will-change: transform` sparingly: each promoted layer costs GPU memory, and 60 layers is acceptable but not free.
- Skip label work entirely on frames where neither the camera matrix nor any tagged object moved.
- Hide labels behind the camera (projected z > 1) and off-screen with `visibility` or `display` toggles batched in the same write pass.
- Occlusion test options:
  - A throttled raycast (every N frames, or via a BVH such as three-mesh-bvh).
  - A depth readback, which is expensive and stalls the GPU, so avoid it.
- Quality trade-off: DOM tags give crisp subpixel text, CSS styling, accessibility, selection and pointer events for free. WebGL SDF text gives depth occlusion, sorting with the scene and no per-label DOM cost, but is blurrier at small sizes, lacks native accessibility and needs its own hit-testing.
- For 20-60 tags with cached sizes and transform-only writes, the DOM approach should cost well under 1 ms per frame in most browsers. This is unverified; profile in Chrome, where the drei slowdown was reported.

### Gaps
- No controlled benchmark of "N DOM labels vs frame time" was found; all numbers above are forum anecdotes.
- web.dev's layout-thrashing / avoid-large-complex-layouts article was not fetched in this session.

## OffscreenCanvas + Web Worker rendering

### Takeaway
WebGL in an OffscreenCanvas worker works in Chrome, Firefox and Safari 17+, but Safari needs macOS Sonoma / iOS 17 or later, and there are reported Safari bugs. The gain is isolation: main-thread jank (DOM labels, React, data parsing) no longer stalls rendering, and vice versa. It does not make the GPU faster. The cost is that the worker has no DOM, so OrbitControls needs forwarded pointer and keyboard events. The kit's DOM labels would also have to get projected positions posted back from the worker, which adds a frame of latency between the 3D view and the tags.

### Cited Findings
- Safari OffscreenCanvas with WebGL requires macOS Sonoma, iPadOS 17 or iOS 17. Safari 17.1 on macOS 13 Ventura does not support WebGL in workers. Safari 16.4 shipped 2D-only OffscreenCanvas, which broke naive WebGL feature detection; the advice is to feature-detect inside the worker. — [WebKit Bugzilla 183720](https://bugs.webkit.org/show_bug.cgi?id=183720)
- A May 2024 WebKit bug reports OffscreenCanvas rendering not working in some multi-worker cases on Safari 17.4.1, with resolution unknown. — [webkit-unassigned: Bug 274301](https://lists.webkit.org/pipermail/webkit-unassigned/2024-May/1165881.html)
- The web.dev OffscreenCanvas article shows three.js in a worker by passing the transferred canvas to the renderer. It notes that three.js expects `canvas.style.width/height`, so the worker needs a fake style object. The article is older. — [web.dev: OffscreenCanvas](https://web.dev/articles/offscreen-canvas)
- The @react-three/offscreen README states that OffscreenCanvas + WebGL is not supported in Safari. This is outdated: Safari 17 shipped it. — [npm @react-three/offscreen](https://www.npmjs.com/package/@react-three/offscreen)

### Inferences
- The three.js manual "OffscreenCanvas" article uses an ElementProxy that forwards events, so OrbitControls can run in the worker against a fake element. That page returned 404 at the URLs tried, so this comes from background knowledge.
- For this kit, the jank sources are likely the main thread's own work (label writes, collision, app UI), which a worker does not remove. The worker only stops them from blocking the canvas. DOM tags must still be positioned on the main thread from posted coordinates, so tags may lag the 3D view by one frame during camera motion, which is visible as swimming.
- Worth it only if main-thread long tasks are proven to be the cause of dropped frames. Otherwise fix the long tasks first.

### Gaps
- No measured jank-isolation gains for three.js workers were found.
- The three.js manual OffscreenCanvas page could not be fetched.

## WebGPURenderer (r16x to r18x): status, fallback, performance, Safari

### Takeaway
WebGPURenderer (`three/webgpu`, TSL and node materials) auto-falls back to a WebGL2 backend, and WebGPU now ships in all major browsers, including Safari 26 (September 2025). For a CPU-bound scene with many separate meshes, though, it is currently **slower** than WebGLRenderer: about 2x CPU time per frame and 5-10x slower first frame in an r183 benchmark, on both backends. For a twin like this (standard materials, PMREM, one shadow, moderate draw calls, DOM labels), there is no expected win today. Stay on WebGLRenderer unless you need compute or TSL-specific features.

### Cited Findings
- Migration guide entries:
  - r171: use `three/webgpu` imports for WebGPURenderer and NodeMaterial.
  - r181: `renderAsync()` and `computeAsync()` deprecated (use init then sync calls).
  - r182: `colorBufferType` renamed to `outputBufferType`.
  - r183: `WebGLCubeRenderTarget` can't be used with WebGPURenderer (use `CubeRenderTarget`), and WebGPU shadows were improved, with a bias retune possibly needed.
  — [three.js Migration Guide](https://github.com/mrdoob/three.js/wiki/Migration-Guide)
- r183 benchmark (4,000 meshes, about 8M triangles, 200 shared geometries, Chrome 148, RTX 4090 Laptop), CPU time in render():

  | Renderer | CPU time per frame | Cold first frame |
  |---|---|---|
  | WebGLRenderer | 4.5 ms | 32 ms |
  | WebGPURenderer (WebGPU backend) | 9.7 ms | 167 ms |
  | WebGPURenderer (forceWebGL) | 10.3 ms | 344 ms |

  The cause was attributed to per-object UBO and bind-group management (three.js issue #30560, being worked on). A forum regular advised staying on WebGLRenderer unless WebGPU features are needed. — [three.js forum: WebGPURenderer 2x slower CPU (r183)](https://discourse.threejs.org/t/webgpurenderer-2x-slower-cpu-and-5-10x-slower-first-frame-than-webglrenderer-on-many-mesh-scenes-r183-same-on-both-backends/91904)
- Other reports:
  - The WebGPURenderer WebGL fallback is much slower than WebGLRenderer (r181).
  - Chrome's forceWebGL path is slower than Firefox's.
  - A user reports a performance drop and shadow regression in r182 WebGPU compared with WebGL r170.
  — [WebGPU performance issue](https://discourse.threejs.org/t/webgpu-performance-issue/87939); [forceWebGL Chrome slower](https://discourse.threejs.org/t/three-webgpu-js-with-forcewebgl-webgl-in-chrome-slower-than-webgl-in-firefox/82593); [r182 vs r170](https://discourse.threejs.org/t/webgpu-significant-performance-drop-and-shadow-quality-regression-in-r182-vs-webgl-r170/89322)
- WebGPU browser support (web.dev, 25 Nov 2025):

  | Browser | Platforms and versions |
  |---|---|
  | Chrome/Edge | 113+ on Windows, macOS, ChromeOS. Android 121+ on Android 12+ with Qualcomm/ARM GPUs. Linux in progress |
  | Firefox | 141 on Windows. 145 on macOS Tahoe ARM. Linux, Android and Intel Mac in progress |
  | Safari | macOS Tahoe 26, iOS/iPadOS 26, visionOS 26 |

  — [web.dev: WebGPU is now supported in major browsers](https://web.dev/blog/webgpu-supported-major-browsers)
- `navigator.gpu` in Safari requires macOS Tahoe / iOS 26 or later. Older Apple OS versions get WebGPURenderer's WebGL2 fallback. — [WebKit Bugzilla 299237](https://bugs.webkit.org/show_bug.cgi?id=299237); [gpuweb Implementation Status](https://github.com/gpuweb/gpuweb/wiki/Implementation-Status)
- R3F users discuss WebGPU with WebGL2 fallback and tree-shaking concerns; avoid compute if you rely on the fallback. — [R3F WebGPU, WebGL2 Fallback, Tree Shaking](https://discourse.threejs.org/t/r3f-webgpu-webgl2-fallback-tree-shaking/87188)

### Inferences
- WebGPU helps when:
  - The workload is GPU-compute shaped (particles, GPU culling, simulation).
  - Render bundles can freeze large static draw lists.
  - You want TSL post-processing.
- WebGPU does not help when:
  - CPU per-object overhead dominates, which is currently worse in three.js's implementation.
  - The app relies on `onBeforeCompile` / ShaderMaterial GLSL, EffectComposer passes, or a CSS-label pipeline. Note: ShaderMaterial and onBeforeCompile are not supported in node materials, so customised shaders need TSL ports. This is background knowledge.
- One claimed WebGPURenderer win (background knowledge, unverified): the 2026 three.js line has been adding render-bundle and "BundleGroup" support for static subtrees, which could make the kit's static plant very cheap on the CPU. Re-benchmark at each release.
- One vendor article (vr.org, May 2026) frames WebGPU as "baseline 2026"; treat that as opinion. — [vr.org](https://vr.org/articles/webgpu-baseline-2026-three-js-webxr-default)

### Gaps
- The three.js manual WebGPURenderer page returned 404 at the URLs tried, so official claims (render bundles, compute, "less CPU overhead") could not be quoted.
- No Safari-specific three.js WebGPU vs WebGL benchmark was found.
- Status of issue #30560 as of late 2026 was not checked.

## Pixel ratio and MSAA

### Takeaway
Fragment cost scales with the square of the pixel ratio: DPR 2 is 4x the pixels of DPR 1, and 1.5 is 2.25x. The near-universal practice is `setPixelRatio(Math.min(devicePixelRatio, 2))`. Adaptive apps lower DPR (toward 1-1.5) while the camera moves and restore full DPR when still. MSAA (`antialias: true`) mostly costs bandwidth and memory, and matters less at DPR 2 or higher, where many apps drop it.

### Cited Findings
- Capping at 2 is the common recommendation. Going from 3 to 2 cuts fragment work by more than 55% ((2/3)² ≈ 0.44). An uncapped DPR on 3x screens is a 9x pixel load versus 1x. — [Limit Three.js Pixel Ratio for Retina](https://salivity.github.io/three.js/article/limit-three-js-pixel-ratio-for-retina-performance); [react.doctor: cap device pixel ratio](https://www.react.doctor/docs/rules/react-doctor/three-cap-device-pixel-ratio)
- A simple scene ran at about 40 fps on iOS when using the raw devicePixelRatio. — [Low fps on iOS with devicePixelRatio](https://discourse.threejs.org/t/low-fps-on-ios-mobile-with-pixel-ration-set-as-window-devicepixelratio/4963)
- Official examples use the raw DPR; a PR was opened to document limiting internal resolution. Fractional DPR (for example 1.25 or 1.5 on Windows) has its own pitfalls. — [HiDPI + fractional scaling: pitfalls](https://discourse.threejs.org/t/hidpi-fractional-scaling-performance-pitfalls-and-best-practices/87114)
- r3f scales DPR adaptively (`setDpr(window.devicePixelRatio * current)`) via regress or PerformanceMonitor, typically within a [1, 2] range. — [r3f docs: Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- Changing pixel ratio from an FPS meter is a known pattern, but needs hysteresis to avoid oscillation. — [Changing pixelRatio based on fps](https://discourse.threejs.org/t/changing-pixelratio-based-on-fps-good-or-bad-idea/34563)

### Inferences
- For the kit:
  - Cap DPR at 2 on desktop and 1.5 on mobile or low-end devices.
  - Optionally drop to about 1-1.25 during camera motion, with a debounced restore after damping ends. This is the r3f regress pattern.
  - Note that each `setPixelRatio`/`setSize` reallocates the drawing buffer, so avoid toggling it every frame.
  - The cached shadow map is independent of DPR, so DPR savings apply to the main pass only: the PBR shading with PMREM plus PCF shadow lookups, which is the bulk of fill cost.
- MSAA at DPR 2 is a 4x-sample buffer on a 4x-pixel target. Dropping `antialias` when DPR ≥ 2 is a common trade, with mild edge aliasing on thin pipes and rails. Background knowledge; no measured source fetched.

### Gaps
- No measured MSAA cost numbers for three.js were found in this session.
