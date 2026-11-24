# Adaptive / progressive rendering during camera motion ("degrade while moving, refine when still")

Context: three.js digital-twin kit (40-200 glTF-cloned objects, 2048 PCF shadow map, DOM tags projected each frame, objects moving along paths). Research done 2026-11-23. Tool budget limited this pass; gaps are listed explicitly.

## 1. How production web 3D viewers reduce work during camera motion (APS/Forge, xeokit, That Open, Potree, Cesium, Speckle)

### Takeaway
The clearest, best-documented pattern is xeokit's FastNavPlugin: on any camera matrix change it switches off edges, SAO, PBR, colour textures (optionally transparent objects) and optionally drops canvas resolution (default factor 0.6), then restores everything 0.5 s after the last camera change. Autodesk's viewer instead spreads one frame's draw work over several ~17 ms frames ("progressive rendering") and restarts after each interaction; Speckle formalises a dynamic stage (moving) vs a progressive stage (still, accumulating 16 frames); Cesium avoids rendering at all when idle (requestRenderMode) and defers / coarsens tile loading during motion.

### Cited Findings
**xeokit FastNavPlugin (most directly reusable design)**
- Purpose: "makes interaction feel smoother for large models" by temporarily disabling expensive rendering features while the camera moves or the canvas resizes, then restoring after a delay. Blog dated 20 Dec 2020; no FPS measurements given. — [xeokit blog](https://xeokit.io/blog/smoother-navigation-with-fastnavplugin)
- Hidden while interacting: edges, SAO (ambient shadows), PBR (falls back to Blinn-Phong), colour textures, and optionally transparent objects. "Does not hide X-rayed, selected, highlighted objects." — [FastNavPlugin API docs](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/FastNavPlugin/FastNavPlugin.js~FastNavPlugin.html)
- Defaults (from source v2.6.78): `hideEdges`, `hideSAO`, `hidePBR`, `hideColorTexture`, `delayBeforeRestore` = true; `hideTransparentObjects` = false; `scaleCanvasResolution` = false; `scaleCanvasResolutionFactor` = 0.6; `delayBeforeRestoreSeconds` = 0.5. — [FastNavPlugin.js source on unpkg](https://app.unpkg.com/@xeokit/xeokit-sdk@2.6.78/files/src/plugins/FastNavPlugin/FastNavPlugin.js)
- Docs conflict: `defaultScaleCanvasResolutionFactor` listed as 0.6 in constructor docs but 1.0 in getter/setter docs; in source it is undefined unless configured, and restore uses `default || original || 1.0`. — [API docs](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/FastNavPlugin/FastNavPlugin.js~FastNavPlugin.html); [source](https://app.unpkg.com/@xeokit/xeokit-sdk@2.6.78/files/src/plugins/FastNavPlugin/FastNavPlugin.js)
- Mechanism: trigger is `scene.camera.on("matrix", switchToLowQuality)`; each call resets a countdown timer (`delayBeforeRestoreSeconds*1000`); a `scene.on("tick")` handler subtracts `deltaTime` and calls `switchToHighQuality()` when timer <= 0. Low quality calls renderer setters `setColorTextureEnabled`, `setPBREnabled`, `setSAOEnabled`, `setTransparentEnabled`, `setEdgesEnabled` and sets `scene.canvas.resolutionScale`. Optional `onMoved`/`onStopped` callbacks. — [source](https://app.unpkg.com/@xeokit/xeokit-sdk@2.6.78/files/src/plugins/FastNavPlugin/FastNavPlugin.js)
- Blog example uses factor 0.5, described as rendering 75% fewer pixels, "slightly blurry". The 0.5 s delay "prevents rapid flipping between low and high quality". — [xeokit blog](https://xeokit.io/blog/smoother-navigation-with-fastnavplugin)
- The newer xeokit SDK (v3) has an off-thread frustum and solid-angle culler with a `solidAngleLimit`: objects that subtend less than the limit are culled. — [xeokit SDK spatial.culling](https://xeokit.github.io/sdk/docs/api/modules/spatial.culling.html) (from search snippet; page not opened)

**Autodesk APS (Forge) Viewer**
- `optimizeNavigation`: boolean, "Toggles whether the navigation should be optimized for performance. Set to true for mobile devices." — [APS Viewer v7 reference](https://aps.autodesk.com/en/docs/viewer/v7/reference). Note: search snippets disagreed on its default (false in one, true + "Toggles whether progressive rendering is used" in another); treat as unverified.
- Progressive rendering can be turned off per profile (`progressiveRendering: false`); the AEC profile has it off. — [APS Profiles guide](https://aps.autodesk.com/en/docs/viewer/v7/developers_guide/advanced_options/profiles)
- End-user settings: "Progressive display: Uses incremental view updates for more responsive interaction with the model", best combined with "Target Frame Rate" (higher = smoother navigation "but may cause some flickering"); "Disable shadow effect: Provides a faster response, but reduces visual quality during navigation." — [Autodesk Viewer settings help](https://help.autodesk.com/cloudhelp/ENU/ADSKVIEWER-Help/files/AutodeskViewerTools/ADSKVIEWER_Help_AutodeskViewerTools_ViewerSettings_html.html)
- How progressive rendering works (Autodesk engineers, InfoQ talk): each frame targets 1/60 s (~17 ms); if the scene does not fit, the partially drawn image is kept and drawing continues next frame; input is handled after each budgeted frame rather than waiting for a full redraw; after an interaction the render restarts from scratch. Geometry streaming orders chunks by visibility and estimated on-screen size (largest first). No measurements given. — [InfoQ: Autodesk Forge Viewer](https://www.infoq.com/presentations/autodesk-forge-viewer)
- A newer APS "Smooth Navigation" beta (v7.99+) targets "mesh popping and flickering" during navigation. — [APS blog 14 Aug 2024](https://aps.autodesk.com/node/3443) (snippet only)
- Navisworks (desktop sibling) has a long-standing "Adjust Scene Rendering During Navigation" / guaranteed frame rate feature that drops geometry to hold frame rate. — [Navisworks help](https://help.autodesk.com/cloudhelp/2020/ENU/Navisworks/files/GUID-1A87025C-F276-4978-847D-52CD63AF3F29.htm) (title only)

**Speckle viewer**
- Pipeline has three stages, each a list of `GPass`: `dynamicStage` (camera moving), `progressiveStage` (stationary, accumulating samples), `passthroughStage` (re-show finished progressive result without restarting). `accumulationFrameCount` defaults to 16. Callbacks `onStationaryBegin()`, `onStationaryEnd()`, `onAccumulationComplete()`. — [Speckle Progressive Pipeline API](https://docs.speckle.systems/developers/viewer/rendering-pipeline-api/progressive-pipeline-api)

**Cesium**
- `requestRenderMode` (since 1.42): render only when something changed (camera move, time, tile load, etc.); `scene.requestRender()` for changes Cesium can't detect; `maximumRenderTimeChange` default 0.0 (set Infinity if nothing time-dependent). Idle CPU went from 25.1% to 3.0% in Chrome on an i7 laptop; idle FPS becomes 0. — [Cesium blog, 2018](https://cesium.com/blog/2018/01/24/cesium-scene-rendering-performance/)
- 3D Tiles motion-related knobs (Cesium3DTileset): `maximumScreenSpaceError` 16; `dynamicScreenSpaceError` true (coarser tiles far away in horizon views; density 2.0e-4, factor 24, heightFalloff 0.25); `foveatedScreenSpaceError` true (centre of screen first, SSE relaxed at edges); `foveatedTimeDelay` 0.2 s "how long to wait after the camera stops before loading deferred tiles"; `cullRequestsWhileMoving` true with multiplier 60; `progressiveResolutionHeightFraction` 0.3 (first load tiles that meet SSE at reduced resolution); `cacheBytes` 512 MiB. Docs conflict on `foveatedConeSize` (0.3 vs 0.1). — [Cesium3DTileset reference](https://cesium.com/learn/cesiumjs/ref-doc/Cesium3DTileset.html)

**Potree**
- Octree LOD: frustum-culls regions and renders distant regions at lower LOD under a global point budget; adaptive point size to hide holes; default root spacing = bbox diagonal / 250. — [Schütz 2016 thesis](https://www.cg.tuwien.ac.at/research/publications/2016/SCHUETZ-2016-POT/); [Schütz CESCG 2015](https://www.old.cescg.org/CESCG-2015/papers/Schutz-Rendering_Large_Point_Clouds_in_Web_Browsers.pdf)

**Other analogues**
- HOOPS Web Viewer aims for a minimum interactive framerate by interrupting rendering and returning control to the user. — [Tech Soft 3D HOOPS performance guide](https://docs.techsoft3d.com/hoops/visualize-web/prog_guide/viewing/performance.html) (snippet)

### Inferences
- For a 40-200 object twin, Autodesk-style "draw part of the scene per frame" is overkill (it exists for 10^5-10^6 fragments). The xeokit model maps almost 1:1 onto three.js: listen to OrbitControls `change` (or compare camera matrixWorld each frame), enter fast mode, restart a ~300-500 ms timer, restore on expiry. The three.js equivalents of xeokit's toggles are: `renderer.setPixelRatio`, `renderer.shadowMap` freeze/off, skip SSAO/outline/edges post passes, swap MeshStandardMaterial for MeshLambert/Basic (or lower envMap), hide transparent meshes, hide `LineSegments` edge overlays.
- xeokit deliberately keeps selected/highlighted/x-rayed objects visible in fast mode; a twin should likewise never hide the selected asset, alarmed assets or tagged objects.
- Cesium's `foveatedTimeDelay` 0.2 s and xeokit's 0.5 s bracket a sensible restore delay range.

### Gaps
- That Open Engine (IFC.js successor): only found that `CullerRenderer` is a GPU visibility-buffer culler ([docs](https://docs.thatopen.com/api/@thatopen/components/classes/CullerRenderer)); I could not confirm its config fields (threshold, update-on-camera-rest behaviour) in this pass.
- APS `optimizeNavigation` default and exactly what it disables (AO? AA?) could not be confirmed from primary docs; search snippets disagree.
- Speckle docs don't say which passes run in the dynamic stage or how "stationary" is detected.
- Potree default `pointBudget` value and any motion-specific behaviour not confirmed.
- No vendor published fps/ms gains for these motion modes.

## 2. Hiding small/detail objects while moving; popping and how to hide it

### Takeaway
Screen-size (solid-angle / projected-size) culling is used by xeokit v3 (`solidAngleLimit`), Autodesk (stream/draw by estimated screen size) and Cesium (screen-space error), but no source quantified popping for motion-only culling. Popping mitigations documented elsewhere are dithered (screen-door) fades over ~0.5 s and staged alpha blends; three.js has no built-in screen-size culler (`THREE.LOD` is distance-based).

### Cited Findings
- xeokit v3 culler drops objects whose solid angle is below `solidAngleLimit`. — [xeokit SDK spatial.culling](https://xeokit.github.io/sdk/docs/api/modules/spatial.culling.html)
- Autodesk orders geometry by visibility and estimated on-screen size, largest first. — [InfoQ](https://www.infoq.com/presentations/autodesk-forge-viewer)
- Unreal Niagara scalability culls effects whose bounds take less than a fraction of screen area, or that have gone a set time without being rendered. — [Unreal docs (search snippet)](https://docs.unrealengine.com/4.27/en-US/API/Plugins/Niagara/FNiagaraSystemScalabilitySetting-/MaxTimeWithoutRender/index.html)
- Cesium for Unreal: dithered opacity masking for LOD transitions, default transition length 0.5 s; dithering has "essentially no overdraw", no depth sort, no extra lighting cost. — [Cesium blog 2022](https://cesium.com/blog/2022/10/20/smoother-lod-transitions-in-cesium-for-unreal/)
- Caveats: Unreal bug where the outgoing LOD pops to full opacity just before the dither completes ([UE-40065](https://issues.unrealengine.com/issue/UE-40065)); dither transitions can lower framerate due to overdraw in transition regions ([Unreal forum](https://forums.unrealengine.com/t/dithered-lod-transition-on-grass-drops-frame-rate/404938/3)).
- Alpha-blend approach: fade LOD k+1 in until both visible, then fade LOD k out; needs depth sorting. — [Scherzer 2008](https://www.cg.tuwien.ac.at/research/publications/2008/SCHERZER-2008-FSR/SCHERZER-2008-FSR-draft.pdf)
- Unity cross-fade LOD uses a dither shader driven by a fade value. — [Unity manual](https://docs.unity3d.com/Manual/lod/lod-transitions-lod-group.html)
- Autodesk's 2024 "Smooth Navigation" beta exists specifically because navigation caused "mesh popping and flickering". — [APS blog](https://aps.autodesk.com/node/3443)
- APS Viewer target-frame-rate trade-off: higher target = smoother but "may cause some flickering". — [Autodesk help](https://help.autodesk.com/cloudhelp/ENU/ADSKVIEWER-Help/files/AutodeskViewerTools/ADSKVIEWER_Help_AutodeskViewerTools_ViewerSettings_html.html)

### Inferences
- Hiding objects mid-orbit is the most visible degradation (Autodesk had to ship a feature to reduce it). With only 40-200 objects the draw-call saving is small; prefer degrading cost per pixel (DPR, shadows, post) over hiding objects. If used, restrict to explicit "detail" priority classes (bolts, cable trays, small sensors) under a projected-size threshold (e.g. bounding-sphere radius in pixels < N), and never hide selected/alarmed/tagged items.
- Cheap three.js popping mitigation: per-material `onBeforeCompile` screen-door dither on an `opacity`-like uniform (no sorting needed), animated over ~200-500 ms; or simply restore during the restore delay while the camera is still (pop happens when the eye is not tracking motion).

### Gaps
- No measurements found of how noticeable motion-only small-object culling is, or of gains in three.js.
- three.js forum LOD-blending threads ([1](https://discourse.threejs.org/t/blend-geometry-in-between-lod/87453), [2](https://discourse.threejs.org/t/how-can-i-achieve-a-blended-look-between-lods/30901)) were not opened.

## 3. Dynamic resolution (pixel ratio) during motion: drei AdaptiveDpr, PerformanceMonitor, r3f regress()

### Takeaway
react-three-fiber has a built-in "regression" mechanism: `performance.regress()` (called on controls `change`) drops `performance.current` to `min` (default 0.1, often set 0.5) and returns to `max` after `debounce` (200 ms); drei's `<AdaptiveDpr>` sets `dpr = initialDpr * current` (optionally `image-rendering: pixelated`). Separately, `<PerformanceMonitor>` adjusts quality by measured fps (not motion) with hysteresis.

### Cited Findings
- r3f store defaults: `performance: { current: 1, min: 0.1, max: 1, debounce: 200, regress }`; "Mere calls to `regress()` will not change or affect anything!" — the app must read `current`. Override e.g. `<Canvas performance={{ min: 0.5 }}>`. Trigger: `controls.addEventListener('change', regress)`. Example responder: `setDpr(window.devicePixelRatio * current)`. The docs' movement-regression example lowers DPR while moving and skips AO until still. — [R3F Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- On-demand rendering: `<Canvas frameloop="demand">` + `invalidate()` on controls change; `invalidate()` only requests a frame. — [R3F Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- drei `AdaptiveDpr`: on `current` change sets pixel ratio to `current * initialDpr`; with `pixelated`, sets canvas `imageRendering` to `pixelated` when current < 1 and `auto` at 1; restores initial DPR on unmount. — [drei AdaptiveDpr source](https://raw.githubusercontent.com/pmndrs/drei/master/src/core/AdaptiveDpr.tsx)
- drei `PerformanceMonitor` defaults: `ms` 250 (sampling window), `iterations` 10, `threshold` 0.75 (fraction of iterations that must be outside bounds), `bounds` e.g. `refreshrate > 90 ? [50, 90] : [50, 60]`, `flipflops` Infinity (then `onFallback`), `factor` 0.5 start, `step` 0.1. Fires `onIncline` above upper bound, `onDecline` below lower; the band between bounds is hysteresis. — [drei PerformanceMonitor](http://drei.docs.pmnd.rs/performances/performance-monitor)
- Typical usage: `onChange={({factor}) => setDpr(round(0.5 + 1.5*factor, 1))}`, `flipflops={3}` then fallback to low quality. — [R3F Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)
- xeokit: resolution factor 0.5 = 75% fewer pixels, "slightly blurry". — [xeokit blog](https://xeokit.io/blog/smoother-navigation-with-fastnavplugin)

### Inferences
- Plain three.js equivalent: on controls `change`, `renderer.setPixelRatio(base * 0.5..0.75)`; on timer expiry restore. Caveat: `setPixelRatio` resizes the drawing buffer (and any EffectComposer targets must be resized too), which reallocates GPU memory; flipping every 200 ms can itself cause a hitch. Alternative that avoids reallocation: keep render targets at full size and render into a sub-viewport (`setViewport`/scissor) then upscale — not verified in a source here.
- Combine both: motion-driven regression (immediate, predictable) plus a PerformanceMonitor-style fps governor to choose the "still" DPR per device.
- Pixel-ratio drop mostly helps fill-bound costs (PCF shadow lookups, SSAO, high-DPR displays); it does nothing for CPU-side costs like projecting DOM tags each frame.

### Gaps
- No published ms/fps numbers for AdaptiveDpr.
- `AdaptiveEvents` (drei; disables raycasting events during regression) not verified in this pass.

## 4. Temporal accumulation / progressive refinement when still

### Takeaway
The pattern is: render a cheap frame while moving, and once still, accumulate N jittered/stochastic frames (Speckle default 16) into a buffer, then stop rendering. Autodesk's progressive rendering resets on each interaction. three-gpu-pathtracer follows the same reset-on-camera-move, accumulate-when-still loop.

### Cited Findings
- Speckle: progressive stage starts at the stationary point, `accumulationFrameCount` 16, then passthrough (no re-render until change). — [Speckle docs](https://docs.speckle.systems/developers/viewer/rendering-pipeline-api/progressive-pipeline-api)
- Autodesk: partial image reused frame to frame; restarts from scratch after interaction. — [InfoQ](https://www.infoq.com/presentations/autodesk-forge-viewer)
- three-gpu-pathtracer README now documents a WebGPU `WebGPUPathTracer` (`setScene(scene, camera)`, `renderSample()` each frame; "not yet stable"); the WebGL `WebGLPathTracer` options (e.g. low-res while moving, fade-in) are in its API docs, not the README. — [three-gpu-pathtracer](https://github.com/gkjohnson/three-gpu-pathtracer)
- Progressive AA exists as a product feature in iJewel3D ("anti-aliasing progressive"). — [iJewel docs](https://docs.ijewel3d.com/playground/anti-aliasing-progressive.html) (title only)

### Inferences
- For the twin, a progressive "still" pipeline could accumulate SSAA/TAA (three.js `TAARenderPass` / `SSAARenderPass` with `sampleLevel`) and high-sample SSAO over ~8-16 frames, then stop rendering until camera, path animation or data changes. But objects moving along paths keep the scene non-stationary, so accumulation would constantly reset unless moving objects are composited separately; practical approach: accumulate only when nothing animates (or the user pauses animation).

### Gaps
- Could not confirm WebGLPathTracer option names/defaults (e.g. `dynamicLowRes`, `lowResScale`, `renderDelay`, `fadeDuration`, `minSamples`) from primary docs in this pass.
- three.js TAARenderPass accumulate-when-still behaviour not checked.

## 5. Freezing shadow map updates / skipping post-processing while moving

### Takeaway
three.js supports `renderer.shadowMap.autoUpdate = false` + `shadowMap.needsUpdate = true` (and per-light `light.shadow.autoUpdate/needsUpdate`). A directional/spot shadow map is rendered from the light, so camera orbit alone does not invalidate it; it only needs updating when casters or lights move. Post-processing (SSAO, outlines/edges, AA) is the standard thing to skip during motion (xeokit SAO/edges; R3F docs skip AO).

### Cited Findings
- Render shadows once: `renderer.shadowMap.autoUpdate = false; renderer.shadowMap.needsUpdate = true;` re-flag only on change. — [three.js forum](https://discourse.threejs.org/t/how-to-render-shadow-map-one-time/7176); drei `BakeShadows` does the same — [drei docs](https://drei.docs.pmnd.rs/performances/bake-shadows)
- Per-light `light.shadow.autoUpdate` / `needsUpdate`. — [shadow guide (skills.sh, secondary)](https://skills.sh/openaec-foundation/three.js-claude-skill-package/threejs-impl-shadows)
- Shadow map cost anecdote: "Shadowmap halves my fps" thread; one user saw ~60 to ~20 fps when updating. — [three.js forum](https://discourse.threejs.org/t/shadowmap-halves-my-fps/11519)
- With constantly moving casters you would need `needsUpdate` almost every frame, defeating the purpose. — [three.js forum](https://discourse.threejs.org/t/many-lights-nearly-everything-static-shadows/9803)
- Version caveat: reports that since r164 the trick of toggling `shadowMap.enabled` off after first render no longer caches the map. — [three.js forum](https://discourse.threejs.org/t/when-three-js-version-bigger-than-164-the-shadowmap-cant-be-cache-this-leads-to-a-decrease-in-rendering-performance/67687) (not verified against source)
- Gotcha: changing `shadow.mapSize` while autoUpdate is false needs `light.shadow.map.setSize(...)`. — [three.js forum](https://discourse.threejs.org/t/shadowmap-not-resizing-correctly-when-scene-autoupdate-false/34474)
- xeokit hides SAO and edges while moving; Autodesk offers "Disable shadow effect" during navigation. — [xeokit docs](https://xeokit.github.io/xeokit-sdk/docs/class/src/plugins/FastNavPlugin/FastNavPlugin.js~FastNavPlugin.html); [Autodesk help](https://help.autodesk.com/cloudhelp/ENU/ADSKVIEWER-Help/files/AutodeskViewerTools/ADSKVIEWER_Help_AutodeskViewerTools_ViewerSettings_html.html)

### Inferences
- For the twin: with a fixed directional light and a fixed shadow camera covering the plant, orbiting does not need shadow re-renders at all. Use `autoUpdate=false` and set `needsUpdate` only when path-moving objects have moved, throttled (e.g. every 2-4 frames, or every frame only when still, and frozen during camera motion). This removes a whole scene pass (all casters) on most frames. If the shadow camera follows the view (fitted frustum), this breaks; keep it static.
- Using `autoUpdate=false` is preferable to toggling `castShadow` or `shadowMap.enabled`, which change material programs and can trigger shader recompiles (a stutter source) — inference, not sourced.
- Projecting DOM tags each frame is CPU/layout cost; during motion one could throttle tag updates or use CSS transforms only — not covered by sources here.

### Gaps
- No primary three.js docs fetched for `WebGLShadowMap.autoUpdate`; no WebGPURenderer equivalent verified.

## 6. Reported measurements of gains

### Takeaway
Hard numbers are scarce: Cesium idle CPU 25.1% to 3.0% with requestRenderMode; xeokit's 0.5 resolution = 75% fewer pixels (arithmetic, not timing); a forum report of shadow-map updates taking a scene from ~60 to ~20 fps. No vendor published per-frame ms savings for its fast-nav mode.

### Cited Findings
- Cesium: 25.1% to 3.0% idle CPU; idle fps 0. — [Cesium blog](https://cesium.com/blog/2018/01/24/cesium-scene-rendering-performance/)
- xeokit: factor 0.5 → 75% fewer pixels; no FPS numbers in the post. — [xeokit blog](https://xeokit.io/blog/smoother-navigation-with-fastnavplugin)
- Shadow map: "halves my fps"; ~60 → ~20 fps on update in one report. — [three.js forum](https://discourse.threejs.org/t/shadowmap-halves-my-fps/11519)
- Autodesk: target 1/60 s per frame, no measured numbers in the talk. — [InfoQ](https://www.infoq.com/presentations/autodesk-forge-viewer)
- R3F guidance: keep draw calls to a few hundred, "no more than 1000 as the very maximum". — [R3F Scaling performance](https://r3f.docs.pmnd.rs/advanced/scaling-performance)

### Inferences
- The kit will need its own benchmarks (renderer.info, GPU timer queries via `EXT_disjoint_timer_query_webgl2`) per toggle: DPR, shadow freeze, post skip, edge hide, small-object hide.

### Gaps
- No independent benchmarks found for AdaptiveDpr, FastNavPlugin or APS progressive rendering.
