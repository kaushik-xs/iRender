# irender-flow-kit

Node-and-edge diagrams for [iRender](../../README.md) documents: flowcharts, workflows and ER diagrams. `ui-flow` reads nodes and edges in [React Flow](https://reactflow.dev/)'s JSON shape, so a graph React Flow saved renders as it is. Every node type is a template you write as an ordinary iRender tree, so a node can hold any component, including [irender-ui-kit](../irender-ui-kit/README.md)'s, icons and sparklines. The kit's script handles pan and zoom, automatic layout, edge routing, selection, dragging, connecting, deleting, a palette and the minimap. It is about 19 KB gzipped (unminified), needs no other library, and is added once per page that has a flow.

```rust
let mut engine = irender_html::Engine::new();
irender_ui_kit::register(&mut engine.registry);      // optional
irender_flow_kit::register(&mut engine.registry);
```

From the CLI, add `--flow-kit` to `validate`, `render` or `components`:

```sh
cargo run -p irender-cli -- render examples/flow/gallery.yaml --data examples/flow/gallery.data.json \
  --ui-kit --flow-kit -o flow.html
```

## React Flow JSON as it is

```yaml
- type: ui-flow
  props: { nodes: "{{ graph.nodes }}", edges: "{{ graph.edges }}", height: 400 }
```

Without templates, every node is drawn by the default node, which shows `data.label` with a target port and a source port. `input` nodes have no target port and `output` nodes no source port, as in React Flow.

The kit reads these fields:

- **Nodes:** `id`, `type`, `position`, `data`, `parentId` (the position is relative to the parent), `hidden`, `className`, `style`, and `width` / `height` (used as minimum sizes).
- **Edges:** `id`, `source`, `target`, `sourceHandle`, `targetHandle`, `type`, `label`, `animated`, `markerEnd` / `markerStart` (`arrow`, `arrowclosed`, or `{ type }`), `className`, `style` (stroke properties) and `hidden`.

Other fields are kept and ignored.

React Flow's own edge types pick the path: `default` and `simplebezier` draw a bezier, and `straight`, `step` and `smoothstep` draw those shapes.

## Node types are templates

```yaml
- type: ui-flow
  props: { nodes: "{{ wf.nodes }}", edges: "{{ wf.edges }}", layout: layered }
  children:
    - type: ui-flow-node
      props: { type: if }
      children:
        - { type: ui-badge, props: { text: If } }
        - { type: text, props: { value: "{{ node.data.label }}" } }
        - { type: ui-flow-port, props: { type: target } }
        - { type: ui-flow-port, props: { id: "true", side: right } }
        - { type: ui-flow-port, props: { id: "false", side: bottom } }
    - type: ui-flow-node            # one port per case, each on its own row
      props: { type: switch }
      children:
        - type: list
          props: { items: "{{ node.data.cases }}", as: c }
          children:
            - type: element
              style: { position: relative, padding: 4px 12px }
              children:
                - { type: text, props: { value: "{{ c.label }}" } }
                - { type: ui-flow-port, props: { id: "{{ c.handle }}", side: right } }
```

How templates work:

- **Which template a node gets:** a `ui-flow-node` draws every node whose `type` matches its own. One without a `type` (or with `type: default`) draws the rest.
- **What a template can read:** inside a template, `node` is the node and `nodeIndex` is its position in the array. Templates can also read state and server data, like any other part of the document.
- **Styling:** the template's `style`, `class`, `attrs` and `on` apply to the node's box. `variant: card` (the default) adds a border, background and padding; `variant: plain` leaves all of that to you.
- **Selection styling:** a selected node has `data-selected`, and one dimmed by `highlight` has `data-dimmed`. Style them with `"&[data-selected]": { … }`.

`ui-flow-port` marks where edges attach:

| prop | |
|---|---|
| `id` | Matched against an edge's `sourceHandle` / `targetHandle`. It can use the node, e.g. `{{ c.handle }}`. An edge with no handle uses the first port of the right type. |
| `type` | `source` (default) or `target`. |
| `side` | `left`, `right`, `top` or `bottom`. Leave it out and it follows the flow's `direction`: for `LR`, targets go on the left and sources on the right. |
| `offset` | How far along the side, e.g. `30%` or `24` (px). Leave it out and ports sharing a side of the node are spread evenly. |

A port is placed on the side of its nearest positioned ancestor. For a port directly in the node, that's the node itself, and its edges start at the node's border. A port inside an element with `position: relative`, such as a table row, sits on that element instead. That's how one node can have a port per row.

## Edge types

```yaml
- type: ui-flow-edge
  props: { marker: arrowclosed }                 # every edge without a template of its own
- type: ui-flow-edge
  props: { type: retry, path: step, marker: arrowclosed }
  style: { stroke: "#dc2626", stroke-dasharray: "4 3" }
  parts: { label: { style: { color: "#dc2626" } } }
  children:                                       # the label: any tree, with `edge` in scope
    - { type: ui-button, props: { label: Retry, size: sm }, on: { click: { action: call, name: retry, args: { id: "{{ edge.id }}" } } } }
```

Edge templates have these props:

- **`path`:** `bezier`, `straight`, `step` or `smoothstep`.
- **`marker` / `markerStart`:** `none`, `arrow` or `arrowclosed`.
- **`animated`:** moving dashes.
- **`floating`:** the edge attaches to the two boxes' borders, facing each other, instead of to ports. This suits ER diagrams.

A template's `style` styles the path. Its children are the label, which sits at the middle of the path. Without children, the label is `edge.label`.

## `ui-flow`

| prop | |
|---|---|
| `nodes`, `edges` | Arrays in React Flow's shape. From state, the browser rebuilds them whenever they change, keyed by id: a node that stays keeps its DOM, including input values and the state of the components inside it. |
| `layout` | `layered` places the top-level nodes in ranks along `direction`, ignoring their positions; the layout also updates when the nodes or their sizes change. Default: use `position`. |
| `direction` | `LR` (default), `TB`, `RL` or `BT`. |
| `nodeGap`, `rankGap` | Layout spacing (40, 80). |
| `nodeOrigin` | Which point of the node `position` names: `[0, 0]` top left (default), `[0.5, 0]` top centre, … |
| `edgePath` | The path for edges whose type and template don't choose one (default `bezier`). |
| `fit`, `fitPadding` | Fit the view to the nodes when they first appear (default true, padding 0.1). Fitting never zooms in past 1. |
| `viewport` | `{ x, y, zoom }` to start at when `fit` is false. React Flow saves this with `toObject()`. |
| `center` | A node id; the view pans to that node whenever this changes. |
| `highlight` | `neighbors`: while a node is selected, everything not connected to it is dimmed. |
| `selectable`, `pan`, `zoom` | Defaults: all true. |
| `editable` | Turns on `draggable`, `connectable` and `deletable` together. Each can still be set on its own. See [Editing](#editing). |
| `draggable`, `connectable`, `deletable` | Default: `editable`. |
| `connect` | Connection rules; see [Editing](#editing). |
| `newEdge` | Fields every new edge starts with, e.g. `{ type: smoothstep, markerEnd: arrowclosed }`. |
| `newNode` | A node (`{ type, data }`) made where a connection is dropped on empty canvas, and joined to it. |
| `confirmDelete` | The element id of a dialog to open before deleting. |
| `wheel` | `zoom` (default) or `pan`, which leaves zooming to Ctrl/⌘ + wheel. |
| `minZoom`, `maxZoom` | 0.1 and 2. |
| `background` | `dots` (default), `lines` or `none`. |
| `controls`, `minimap` | The zoom/fit buttons (default on) and an overview that pans the view when clicked or dragged (default off). Style them through `parts`. |
| `height` | Default 480. The width is the container's. |
| `title` | Accessible name. |
| `bind` | State key with the selected node's id (the last one selected), or null. It works in both directions. |
| `bindSelection` | State key with every selected node and edge, `{ nodes: [ids], edges: [ids] }`, in both directions. |
| `bindNodes` | State key that gets the nodes array after a drag, an added node or a deletion. Point `nodes` at the same key to keep them in sync. |
| `bindEdges` | The same for edges, after a connection or a deletion. |
| `bindDelete` | With `confirmDelete`: what would be deleted, `{ nodes, edges }`, while the dialog is open. Null otherwise. |
| `bindViewport` | State key kept in sync with `{ x, y, zoom }`. |

Every prop except `background`, `controls`, `minimap`, `height` and `title` can use state.

The flow fires these events, which you can handle in `on`. Each fires after its bound state is written, so actions can read it:

- **`nodeselect`:** the selection changed.
- **`nodeopen`:** on a double-click, or Enter on a node that is already selected.
- **`nodemove`:** a node was dragged.
- **`connect`:** an edge was added.
- **`nodeadd`:** a node was added.
- **`deleterequest`:** a deletion is waiting for its dialog.
- **`delete`:** nodes or edges were deleted.

`ui-flow-panel` children (`position`: `top-left` … `bottom-right`) float above the flow without panning or zooming, so toolbars and legends go there.

## Editing

```yaml
- type: ui-flow
  attrs: { id: wf }
  props:
    nodes: "{{ wf.nodes }}"
    edges: "{{ wf.edges }}"
    bindNodes: wf.nodes          # edits are written here …
    bindEdges: wf.edges          # … and here
    editable: true
    connect: { sameParent: true, pairs: [{ source: trigger, target: step }, { source: step }] }
    newEdge: { type: smoothstep }
    newNode: { type: step, data: { label: New step } }
    confirmDelete: confirm-delete
    bindDelete: doomed
- type: ui-flow-item             # a palette entry: drag it onto the flow, or click it
  props: { flow: wf, node: { type: step, data: { label: New step } } }
  children: [{ type: ui-badge, props: { text: Step } }]
```

The kit makes every edit itself and writes the result to `bindNodes` / `bindEdges`; expressions can't build arrays. Point `nodes` and `edges` at the same keys and the flow follows.

- **Connecting:** drag from a port to a port of the other type, starting at either end. Dropping on a node's body joins the first of its ports that the rules allow. While dragging, the port under the pointer turns green or red. Dropping on empty canvas makes a `newNode` there and joins it.
- **Connection rules:** a connection is refused when any rule says no.

  | Rule | What it refuses |
  |---|---|
  | `connect.self` | A node joined to itself, unless set |
  | `connect.unique` | A second edge between the same two ports (set it to false to allow one) |
  | `connect.sameParent` | Two nodes with different parents |
  | `connect.pairs` | Any pair of node types not in this allow-list of `{ source, target }` (either may be left out) |
  | A port's `accepts` | Node types not in this list at the other end |
  | A port's `max` | More edges than this at the port |
  | A port's `connectable: false` | Any new connection at the port |

  A port's rules can use state.
- **Selecting:**
  - Click a node or an edge to select it. Shift, ⌘ or Ctrl + click adds it to the selection.
  - Shift + drag on the background selects every node inside the box.
  - ⌘/Ctrl + A selects all nodes.
  - Dragging a selected node moves the whole selection.
- **Deleting:** Delete or Backspace removes the selection. A deleted node's edges and child nodes go with it. With `confirmDelete`, the flow writes what would go to `bindDelete` and opens that dialog instead. The dialog's buttons answer it with `flow.delete` or `flow.cancel`:

  ```yaml
  - type: ui-alert-dialog
    props: { id: confirm-delete, title: Delete?, description: "{{ doomed.nodes.length }} node(s) and {{ doomed.edges.length }} edge(s)" }
    children:
      - type: ui-dialog-footer
        children:
          - { type: ui-dialog-close, props: { label: Cancel, variant: outline }, on: { click: { action: call, name: flow.cancel, args: { flow: wf } } } }
          - { type: ui-dialog-close, props: { label: Delete, variant: destructive }, on: { click: { action: call, name: flow.delete, args: { flow: wf } } } }
  ```
- **Adding:** `ui-flow-item` takes a `node` (`{ type, data }`, and an `id` to keep). Dragging it onto the flow adds the node where it's dropped; clicking it, or pressing Enter or Space, adds it in the middle of the view. Its `flow` is the flow's element id. It can be left out when the item is inside the flow, or the page has one flow. New ids are the type plus a number, such as `step_3`.

### Commands

A `call` whose name starts with `flow.` is a command to the flow:

| Command | What it does |
|---|---|
| `flow.fit`, `flow.zoomin`, `flow.zoomout` | Fit or zoom the view |
| `flow.delete` | Answers a pending `confirmDelete`, or deletes the selection |
| `flow.cancel` | Drops a pending deletion |
| `flow.add` | Adds `args.node` in the middle of the view, or at `args.position` |
| `flow.select` | Selects `args.id`, or clears the selection when it's null |
| `flow.center` | Pans to `args.id` |
| `flow.relayout` | Clears dragged positions and lays the nodes out again |

Commands from inside the flow, such as a panel button, a node, or a dialog in either, reach it directly. From elsewhere, add `args.flow` with the flow's element id; without it, a command reaches the page's only flow.

## Mouse, touch and keyboard

| Input | What it does |
|---|---|
| Drag the background | Pans the view |
| Wheel or pinch | Zooms |
| Click a node or edge | Selects it (Shift adds to the selection) |
| Click the background | Clears the selection |
| Shift + drag the background | Box selection |
| Drag a node | Moves it, with the rest of the selection, when `draggable` is on |
| Drag from a port | Connects, when `connectable` is on |

Controls inside nodes (buttons, inputs, links) work as usual and don't start a drag. Add `data-flow-nodrag` to any other element that should behave the same way.

The flow and its nodes are focusable:

| Key | What it does |
|---|---|
| Tab | Moves focus to the flow and its nodes |
| Enter / Space | Selects the focused node |
| Escape | Cancels a connection, or clears the selection |
| Delete / Backspace | Deletes the selection, when `deletable` is on |
| ⌘/Ctrl + A | Selects all nodes |
| `+` / `-` | Zoom |
| Arrows | Pan, or move the selection when a node has focus and `draggable` is on (Shift for bigger steps) |

## Safety

The JSON's `style` objects are applied one property at a time, so a value can't escape its property. In documents rendered with `Engine::untrusted()`, values containing `url(` are dropped as well. `className` only adds classes. Labels are text, unless a template renders them.

## Not built yet

See [PENDING.md](../../PENDING.md#flow-kit-next-phases).
