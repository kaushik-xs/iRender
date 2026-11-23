# Examples

Each example shows a different way that parts of a page are dynamic. All commands run from the repository root.

## Browse them all

```sh
examples/build.sh             # renders everything into target/site
python3 examples/serve.py     # open http://localhost:8000
```

The script renders every example, plus an index page linking them, into one folder. The index is itself an iRender document: `index.yaml` with `index.data.json` as its data. To list a new example, add it to `index.data.json` and to `build.sh`. Serve the folder instead of opening the files, because the galleries fetch their APIs and browsers block that for `file://` pages. `serve.py` is a static server that also answers the chart gallery's live endpoints and the UI kit gallery's chat. `python3 -m http.server 8000 -d target/site` serves everything else.

| example | dynamic part comes from | features |
|---|---|---|
| [`ui-kit/gallery.yaml`](ui-kit/gallery.yaml) | browser state + an API | every `irender-ui-kit` component, including the react-select-style `ui-combobox` and the chat components; render with `--ui-kit`, and serve it with `serve.py` for load-as-you-type and the chat's streamed replies |
| [`blocks/`](blocks/) | server data + browser state | whole pages in the style of shadcn/ui's blocks: a dashboard (inset `ui-sidebar`, summary cards, an area chart filtered by a toggle group, a table with selection and paging, a ⌘K command palette), sidebars that collapse to icons or slide away, login and sign-up pages, account settings, a pricing page and a theme customizer (`theme.options`); render with `--ui-kit --chart-kit --icon-kit` |
| [`charts/gallery.yaml`](charts/gallery.yaml) | server data + browser state + an API | `irender-chart-kit`: every ECharts category, charts driven by ui-kit controls, drill-downs, cross-filtering by click, date range, legend and brush, and live polled and streamed data from `serve.py`; render with `--ui-kit --chart-kit` |
| [`flow/`](flow/) | server data + browser state | `irender-flow-kit`: a package's ER diagram and a process canvas, each laid out in several ways (in a card, filling the screen, beside a list, as a map), an editable workflow, containers, a flowchart and plain React Flow JSON; render with `--ui-kit --flow-kit --icon-kit` |
| [`maps/gallery.yaml`](maps/gallery.yaml) | server data + browser state | `irender-map-kit`: markers, popups of any components, clusters, picking a place, dragging markers, shapes and GeoJSON, on OpenStreetMap; build with `GOOGLE_MAPS_API_KEY` set to see Google Maps too; render with `--ui-kit --map-kit` |
| [`twin/gallery.yaml`](twin/gallery.yaml) | browser state + an API | `irender-twin-kit`: 3D worlds with three.js from glTF, OBJ, STL and other models (NVIDIA SimReady, see [`twin/models/CREDITS.md`](twin/models/CREDITS.md)), machine and sensor readouts with limits and alarms, colours by value, paths, popups, live data from `serve.py` and editing a layout; blocks `twin-01` to `twin-03`; render with `--ui-kit --twin-kit` |
| [`icons/gallery.yaml`](icons/gallery.yaml) | server data (the kit's pack listing) + browser state | `irender-icon-kit`: every pack and icon, sizes and stroke widths, icons in buttons, switching icons with `if` / `else`; render with `--ui-kit --icon-kit` and `--data` from `irender icons --json` |
