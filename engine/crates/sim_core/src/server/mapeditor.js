// Map editor of the admin console (#mapedit): paints the tile maps, draws zones and passages, places
// buildings, NPCs and hidden objects, edits the legend (with coloured variants of any tile), maps and borders.
// Edits stay in the page (with undo and a local draft) until saved: GET/PUT /api/admin/map.
// Uses the console's helpers: $, esc, api, put, post, toast, S, C, hashColor, loadImg, sheetOf.
"use strict";
const ME = (() => {
  const TS = 16;
  const st = {
    doc: null, file: "", others: [], layer: 0, leg: {},
    tool: "brush", primary: ".", secondary: ".", brush: 1, round: false, fillAll: false, outline: false,
    undo: [], redo: [], dirty: false, sel: null, area: null, clip: null, pasting: false,
    view: { scale: 1, ox: 0, oy: 0, fitted: false }, hover: null, drag: null, stroke: null, pan: null, space: false,
    areaKinds: { tiles: true, Building: true, Entity: true, Hidden: true, zone: true, portal: true }, pendingPortal: null, terrain: null, tctx: null, built: false, tab: "palette", liveTiles: null,
    show: { grid: true, zones: true, portals: true, buildings: true, npcs: true, hidden: true, solid: false, others: true, labels: true },
    opt: { building: "", owner: "", bname: "", template: "", faction: "", npcName: "", count: 1, hideItem: "", hideQty: 1, wall: "W", floor: "w", door: "d" },
  };
  const TOOLS = [
    ["hand", "✋", "Mano: trascina per spostarti sulla mappa (T, o tieni premuto spazio)", "t"],
    ["select", "↖", "Seleziona e sposta (V)", "v"], ["brush", "✎", "Pennello (B)", "b"], ["eraser", "⌫", "Gomma: dipinge la secondaria (E)", "e"],
    ["line", "╱", "Linea, strade e fiumi (L)", "l"], ["rect", "▭", "Rettangolo (R)", "r"], ["ellipse", "◯", "Ellisse (O)", "o"],
    ["fill", "▧", "Riempimento (F)", "f"], ["pick", "⊙", "Contagocce (I)", "i"], ["room", "⌂", "Stanza: muri, pavimento e porta (Q)", "q"],
    ["area", "⬚", "Selezione: copia, taglia, incolla (M)", "m"], ["zone", "▣", "Nuova zona (Z)", "z"], ["portal", "⇅", "Passaggio fra mappe (P)", "p"],
    ["building", "🏠", "Edificio (H)", "h"], ["npc", "☺", "NPC (N)", "n"], ["hidden", "✦", "Oggetto nascosto (K)", "k"],
  ];
  const SIDES = ["North", "South", "East", "West"];
  const SIDE_IT = { North: "nord", South: "sud", East: "est", West: "ovest" };
  const GROUP_ORDER = ["Pavimenti", "Terreno", "Campi", "Piante", "Acqua", "Muri e arredi", "Sottosuolo", "Altro"];

  // ── Helpers ────────────────────────────────────────────────────────────────
  const L = () => st.doc.map.layers[st.layer];
  const layerIdx = (id) => st.doc.map.layers.findIndex((l) => l.id === id);
  const zones = () => st.doc.map.zones;
  const clampN = (v, a, b) => Math.max(a, Math.min(b, v));
  const opts = (list, cur, none) => (none != null ? `<option value="">${esc(none)}</option>` : "") +
    list.map(([v, t]) => `<option value="${esc(v)}"${v === cur ? " selected" : ""}>${esc(t)}</option>`).join("");
  const tileName = (ch) => st.leg[ch]?.name || st.leg[ch]?.id || ch;
  const walkable = (ch) => st.leg[ch] ? st.leg[ch].walkable !== false : true;
  function rebuildLegend() { st.leg = Object.fromEntries(st.doc.map.legend.map((t) => [t.ch, t])); }
  /** The zone a placement names: an id, a map id or `#tag` (first zone with the tag). */
  function zoneInfo(id) {
    if (!id) return null;
    if (id.startsWith("#")) { const z = zones().find((z) => (z.tags ?? []).includes(id.slice(1))); return z ? zoneInfo(z.id) : null; }
    const z = zones().find((z) => z.id === id);
    if (z) return { layer: layerIdx(z.layer), x: z.rect[0], y: z.rect[1], w: z.rect[2], h: z.rect[3] };
    const li = layerIdx(id);
    return li >= 0 ? { layer: li, x: 0, y: 0, w: st.doc.map.layers[li].width, h: st.doc.map.layers[li].height } : null;
  }
  /** Kind, body and position of a placement (`{Building: {...}}`, `{Entity: {...}}`). */
  function pl(p) {
    const kind = Object.keys(p)[0], b = p[kind];
    if (kind === "Player") return { kind, b, pos: null };
    const z = zoneInfo(b.zone);
    if (!z) return { kind, b, pos: null };
    const random = !b.at;
    const pos = random ? { layer: z.layer, x: z.x + Math.floor(z.w / 2), y: z.y + Math.floor(z.h / 2) } : { layer: z.layer, x: b.at[0], y: b.at[1] };
    return { kind: kind === "Building" && b.hidden && b.building === "nascondiglio" ? "Hidden" : kind, b, pos, random };
  }
  /** Smallest zone of the current map containing a cell (the map itself at worst). */
  function zoneAt(x, y, li = st.layer) {
    const lid = st.doc.map.layers[li].id;
    const z = zones().filter((z) => z.layer === lid && x >= z.rect[0] && y >= z.rect[1] && x < z.rect[0] + z.rect[2] && y < z.rect[1] + z.rect[3])
      .sort((a, b) => a.rect[2] * a.rect[3] - b.rect[2] * b.rect[3])[0];
    return z ? z.id : lid;
  }
  function inZone(zid, li, x, y) {
    const z = zoneInfo(zid);
    return z && z.layer === li && x >= z.x && y >= z.y && x < z.x + z.w && y < z.y + z.h;
  }

  // ── Tile pictures (tinted when the type has a colour) ──────────────────────
  const imgs = {}, arts = {}, urls = {};
  function tileImage(sprite) {
    if (!S.assets.has("tile_" + sprite + ".png")) return null;
    if (!imgs[sprite]) {
      const im = new Image();
      im.onload = () => { for (const k of Object.keys(arts)) if (k.startsWith(sprite + "|")) delete arts[k]; scheduleRebuild(true); };
      im.src = "/assets/tile_" + sprite + ".png"; imgs[sprite] = im;
    }
    return imgs[sprite].complete && imgs[sprite].naturalWidth ? imgs[sprite] : null;
  }
  /** 16×16 canvas of a legend entry. */
  function art(t) {
    const sprite = t?.sprite || t?.id || "?", key = sprite + "|" + (t?.color ?? "");
    if (arts[key]) return arts[key];
    const c = document.createElement("canvas"); c.width = c.height = TS;
    const x = c.getContext("2d"), im = tileImage(sprite);
    if (im) {
      x.drawImage(im, 0, 0, TS, TS);
      if (t.color) {
        x.globalCompositeOperation = "multiply"; x.fillStyle = t.color; x.fillRect(0, 0, TS, TS);
        x.globalCompositeOperation = "destination-in"; x.drawImage(im, 0, 0, TS, TS);
      }
    } else { x.fillStyle = t?.color || hashColor(t?.id ?? "?"); x.fillRect(0, 0, TS, TS); }
    if (im || !S.assets.has("tile_" + sprite + ".png")) arts[key] = c;
    return c;
  }
  function artUrl(t) {
    const key = (t?.sprite || t?.id) + "|" + (t?.color ?? "");
    if (!urls[key] || !arts[key]) urls[key] = art(t).toDataURL();
    return urls[key];
  }
  let rebuildTimer = 0;
  function scheduleRebuild(palette) {
    clearTimeout(rebuildTimer);
    rebuildTimer = setTimeout(() => { buildTerrain(); if (palette && st.tab === "palette") renderTab(); }, 60);
  }

  // ── Terrain canvas of the current map ───────────────────────────────────────
  function buildTerrain() {
    if (!st.doc) return;
    const l = L();
    st.terrain = document.createElement("canvas"); st.terrain.width = l.width * TS; st.terrain.height = l.height * TS;
    st.tctx = st.terrain.getContext("2d");
    for (let y = 0; y < l.height; y++) for (let x = 0; x < l.width; x++) drawCell(x, y);
    draw(); drawMini();
  }
  function drawCell(x, y) {
    const ch = L().grid[y]?.[x];
    st.tctx.clearRect(x * TS, y * TS, TS, TS);
    st.tctx.drawImage(art(st.leg[ch] ?? { id: "?" + ch }), x * TS, y * TS);
  }
  const getTile = (x, y, l = L()) => l.grid[y]?.[x];

  // ── Undo: tile strokes as diffs, everything else as snapshots ──────────────
  function snap(full) {
    const m = st.doc.map;
    const layers = m.layers.map((l) => { const { grid, ...rest } = l; return full ? { ...rest, rows: grid.map((r) => r.join("")) } : rest; });
    return JSON.stringify({ full, layer: st.layer, map: { ...m, layers }, placements: st.doc.placements });
  }
  function restore(s) {
    const d = JSON.parse(s), old = st.doc.map.layers, grids = Object.fromEntries(old.map((l) => [l.id, l.grid]));
    st.doc.map = d.map; st.doc.placements = d.placements;
    st.doc.map.layers.forEach((l, i) => {
      if (d.full) { l.grid = l.rows.map((r) => [...r]); delete l.rows; } else l.grid = grids[l.id] ?? old[i]?.grid ?? blank(l.width, l.height, ".");
    });
    st.layer = clampN(d.layer, 0, st.doc.map.layers.length - 1);
    rebuildLegend();
  }
  /** Runs a change of anything but single tiles, undoable. */
  function change(label, fn, full = false) {
    const before = snap(full);
    const r = fn();
    if (r === false) return false;
    st.undo.push({ kind: "doc", label, before, after: snap(full) }); st.redo = [];
    touched(); return r;
  }
  function beginTiles() { st.stroke = { layer: st.layer, cells: new Map() }; }
  function setTile(x, y, ch) {
    const l = L();
    if (x < 0 || y < 0 || x >= l.width || y >= l.height || !ch) return;
    const old = l.grid[y][x];
    if (old === ch) return;
    const k = x + "," + y;
    if (st.stroke && !st.stroke.cells.has(k)) st.stroke.cells.set(k, [x, y, old]);
    l.grid[y][x] = ch; drawCell(x, y);
  }
  function endTiles(label) {
    const s = st.stroke; st.stroke = null;
    if (!s || !s.cells.size) return;
    const l = st.doc.map.layers[s.layer];
    st.undo.push({ kind: "tiles", label, layer: s.layer, cells: [...s.cells.values()].map(([x, y, old]) => [x, y, old, l.grid[y][x]]) });
    st.redo = []; touched();
  }
  function undoRedo(redo) {
    const a = (redo ? st.redo : st.undo).pop();
    if (!a) { toast(redo ? "Niente da rifare" : "Niente da annullare"); return; }
    (redo ? st.undo : st.redo).push(a);
    if (a.kind === "tiles") {
      const l = st.doc.map.layers[a.layer];
      for (const [x, y, old, nu] of a.cells) l.grid[y][x] = redo ? nu : old;
      if (st.layer !== a.layer) { st.layer = a.layer; fit(); }
    } else { restore(redo ? a.after : a.before); st.sel = null; }
    buildTerrain(); renderAll(); touched();
    toast((redo ? "Rifatto: " : "Annullato: ") + a.label);
  }
  let draftTimer = 0;
  function touched() {
    st.dirty = true; renderStatus();
    clearTimeout(draftTimer);
    draftTimer = setTimeout(() => { try { localStorage.setItem("mapEditorDraft", JSON.stringify({ file: st.file, at: Date.now(), doc: serialize() })); } catch (e) {} }, 1500);
  }

  // ── Loading and saving ──────────────────────────────────────────────────────
  const blank = (w, h, ch) => Array.from({ length: h }, () => Array(w).fill(ch));
  function adopt(doc) {
    for (const l of doc.map.layers) {
      const rows = l.tiles ?? [];
      const w = Math.max(l.width || 0, ...rows.map((r) => [...r].length)), h = Math.max(l.height || 0, rows.length);
      l.width = w; l.height = h;
      l.grid = Array.from({ length: h }, (_, y) => { const r = [...(rows[y] ?? "")]; while (r.length < w) r.push("."); return r; });
      delete l.tiles;
    }
    for (const k of ["zones", "portals", "edges", "networks", "walls", "props", "legend"]) doc.map[k] ??= [];
    doc.placements ??= [];
    return doc;
  }
  function serialize() {
    const m = st.doc.map;
    return {
      map: { ...m, layers: m.layers.map((l) => { const { grid, tiles_file, ...rest } = l; return { ...rest, tiles: grid.map((r) => r.join("")) }; }) },
      placements: st.doc.placements,
    };
  }
  async function load(askDraft = true) {
    const r = await api("/api/admin/map");
    st.file = r.file; st.others = r.others ?? [];
    st.doc = adopt({ map: r.map, placements: r.placements });
    let draft = null;
    try { draft = JSON.parse(localStorage.getItem("mapEditorDraft") || "null"); } catch (e) {}
    if (askDraft && draft?.doc && JSON.stringify(draft.doc) !== JSON.stringify(serialize()) &&
        confirm(`C'è una bozza dell'editor non salvata (${new Date(draft.at).toLocaleString("it-IT")}). Ripristinarla?`)) {
      st.doc = adopt(JSON.parse(JSON.stringify(draft.doc)));
      st.dirty = true;
    } else { st.dirty = false; try { localStorage.removeItem("mapEditorDraft"); } catch (e) {} }
    rebuildLegend();
    st.undo = []; st.redo = []; st.sel = null; st.area = null;
    st.layer = clampN(st.layer, 0, st.doc.map.layers.length - 1);
    const firstWalk = st.doc.map.legend.find((t) => t.walkable !== false);
    if (!st.leg[st.primary]) st.primary = firstWalk?.ch ?? ".";
    if (!st.leg[st.secondary]) st.secondary = firstWalk?.ch ?? ".";
    for (const [k, v] of [["building", Object.keys(C("buildings"))[0]], ["template", Object.keys(C("templates"))[0]], ["hideItem", Object.keys(C("items"))[0]]]) if (!st.opt[k]) st.opt[k] = v ?? "";
    st.view.fitted = false;
    renderLayerSelect(); resize(); buildTerrain(); renderAll();
  }
  async function save(restart, dry) {
    try {
      const r = await put(`/api/admin/map?${dry ? "dry=true" : restart ? "restart=true" : ""}`, serialize());
      if (dry) { showCheck([{ ok: true, text: "Il server accetta la mappa: " + r.ok }, ...(r.warnings ?? []).map((w) => ({ text: w }))], true); return; }
      st.dirty = false; renderStatus();
      try { localStorage.removeItem("mapEditorDraft"); } catch (e) {}
      toast(r.ok);
      try { S.content = await api("/api/content"); } catch (e) {}
      if (restart && typeof loadMap === "function") { await loadMap(); if (typeof refreshAll === "function") refreshAll(); }
      await load(false);
    } catch (e) {
      const errs = String(e.message).split(/\n\s*-\s*/).map((s) => s.trim()).filter(Boolean);
      showCheck(errs.map((text) => ({ bad: true, text })), true);
      toast(dry ? "La verifica ha trovato problemi" : "Salvataggio rifiutato: vedi Verifica", true);
    }
  }
  /** Sends the tiles that differ from the running game (maps with the same id and size). */
  async function applyLive() {
    let live; try { live = await api("/api/ui/map"); } catch (e) { toast(e.message, true); return; }
    let total = 0;
    for (const l of st.doc.map.layers) {
      const ll = live.layers.find((x) => x.id === l.id);
      if (!ll || ll.width !== l.width || ll.height !== l.height) continue;
      const cells = [];
      for (let y = 0; y < l.height; y++) { const row = [...(ll.tiles[y] ?? "")]; for (let x = 0; x < l.width; x++) if (row[x] !== l.grid[y][x]) cells.push([x, y, l.grid[y][x]]); }
      const missing = cells.find((c) => !live.legend[c[2]]);
      if (missing) { toast(`La casella '${missing[2]}' (${tileName(missing[2])}) non esiste ancora nella partita: salva e ricomincia`, true); return; }
      for (let i = 0; i < cells.length; i += 4000) {
        try { await post("/api/commands?now=true", { type: "set_tiles", layer: l.id, cells: cells.slice(i, i + 4000) }); }
        catch (e) { toast(e.message, true); return; }
      }
      total += cells.length;
    }
    toast(total ? `${total} caselle applicate alla partita in corso` : "La partita ha già queste caselle");
  }

  // ── Markup ──────────────────────────────────────────────────────────────────
  function buildUi() {
    if (st.built) return;
    st.built = true;
    const style = document.createElement("style");
    style.textContent = `
#s-mapedit.on { display: grid; grid-template-rows: auto minmax(0, 1fr) auto; gap: 6px; height: calc(100vh - 70px); }
.me-top { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.me-top .sep { width: 1px; height: 22px; background: var(--line); margin: 0 2px; }
.me-body { display: grid; grid-template-columns: 44px minmax(0, 1fr) 372px; gap: 6px; min-height: 0; }
.me-tools { display: flex; flex-direction: column; gap: 3px; overflow: auto; }
.me-tools button { width: 40px; height: 36px; padding: 0; font-size: 17px; }
.me-tools button.on { background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }
.me-canvaswrap { position: relative; background: #0d0f12; border-radius: 8px; overflow: hidden; min-height: 240px; }
.me-canvaswrap canvas { width: 100%; height: 100%; display: block; image-rendering: pixelated; cursor: crosshair; touch-action: none; }
.me-side { display: flex; flex-direction: column; gap: 6px; min-height: 0; overflow-y: auto; }
.me-side > * { flex-shrink: 0; margin-bottom: 0; }
.me-side > #meTab { flex: 1 1 220px; min-height: 220px; overflow: auto; }
.me-side > #meSelBox { max-height: 45%; overflow: auto; }
.me-tabs { display: flex; flex-wrap: wrap; gap: 3px; }
.me-tabs button.on { background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }
.me-pal { display: grid; grid-template-columns: repeat(auto-fill, minmax(38px, 1fr)); gap: 3px; }
.me-pal button { padding: 2px; position: relative; display: flex; flex-direction: column; align-items: center; font-size: 9px; line-height: 1.1; overflow: hidden; }
.me-pal img, img.me-t { width: 28px; height: 28px; image-rendering: pixelated; display: block; }
img.me-t.s { width: 18px; height: 18px; display: inline-block; vertical-align: middle; }
.me-pal button.p { outline: 2px solid var(--accent); }
.me-pal button.s2 { box-shadow: inset 0 0 0 2px #ff8a3d; }
.me-pal button .solid { position: absolute; top: 1px; right: 2px; font-size: 9px; color: #fff; text-shadow: 0 0 2px #000; }
.me-swatch { display: flex; gap: 8px; align-items: center; }
.me-swatch img { width: 32px; height: 32px; image-rendering: pixelated; border: 2px solid var(--accent); border-radius: 4px; }
.me-swatch img.s2 { border-color: #ff8a3d; }
.me-status { display: flex; gap: 14px; flex-wrap: wrap; color: var(--muted); font-size: 12px; min-height: 18px; }
.me-status b { color: var(--ink); font-weight: 600; }
.me-list { max-height: 360px; overflow: auto; border: 1px solid var(--line); border-radius: 6px; }
.me-list div.it { padding: 3px 6px; border-bottom: 1px solid var(--line); cursor: pointer; display: flex; gap: 6px; align-items: center; }
.me-list div.it:hover, .me-list div.it.on { background: var(--chip); }
.me-form { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 4px 8px; align-items: center; }
.me-form input:not([type=checkbox]):not([type=color]), .me-form select, .me-form textarea { width: 100%; min-width: 0; }
.me-mini { width: 100%; image-rendering: pixelated; border-radius: 6px; background: #0d0f12; cursor: pointer; display: block; }
.me-dirty { color: var(--warn); font-weight: 600; }
table.me-leg td { padding: 2px 4px; } table.me-leg input { padding: 1px 4px; }
.me-anchor { display: inline-grid; grid-template-columns: repeat(3, 22px); gap: 2px; } .me-anchor button { padding: 0; height: 20px; }
.me-anchor button.on { background: var(--accent); color: var(--accent-ink); }
.me-check div { padding: 3px 0; border-bottom: 1px solid var(--line); cursor: pointer; }
@media (max-width: 900px) { .me-body { grid-template-columns: 44px minmax(0, 1fr); } .me-side { grid-column: 1 / -1; max-height: none; } #s-mapedit.on { height: auto; } .me-canvaswrap { height: 60vh; } }`;
    document.head.appendChild(style);
    const sec = $("s-mapedit");
    sec.innerHTML = `
<div class="me-top">
  <b>Editor della mappa</b> <select id="meLayer"></select>
  <button id="meNewMap" title="Crea una nuova mappa (un dungeon, un paese, un interno…)">＋ Nuova mappa</button>
  <button id="meGrow" title="Aggiungi (o togli) caselle ai lati di questa mappa">⤡ Amplia</button>
  <span class="sep"></span>
  <button id="meUndo" title="Annulla (Ctrl+Z)">↶</button><button id="meRedo" title="Rifai (Ctrl+Y)">↷</button>
  <button id="meFit" title="Adatta alla finestra (0)">⤢</button><span id="meZoom" class="muted"></span>
  <span class="sep"></span>
  <label title="G"><input type="checkbox" data-show="grid"> griglia</label>
  <label><input type="checkbox" data-show="zones"> zone</label>
  <label><input type="checkbox" data-show="portals"> passaggi</label>
  <label><input type="checkbox" data-show="buildings"> edifici</label>
  <label><input type="checkbox" data-show="npcs"> NPC</label>
  <label><input type="checkbox" data-show="hidden"> nascosti</label>
  <label title="Mostra in rosso le caselle dove non si cammina"><input type="checkbox" data-show="solid"> solidi</label>
  <label title="Piazzamenti degli altri file (sola lettura)"><input type="checkbox" data-show="others"> altri file</label>
  <label><input type="checkbox" data-show="labels"> nomi</label>
  <span class="sep"></span>
  <span id="meDirty"></span>
  <button id="meCheck">Verifica</button>
  <button id="meSave" class="primary" title="Ctrl+S">Salva</button>
  <button id="meSaveRestart" title="Salva e inizia una nuova partita con questa mappa">Salva e ricomincia</button>
  <button id="meLive" title="Manda alla partita in corso le caselle cambiate (non zone, edifici o NPC)">Applica caselle alla partita</button>
  <button id="meMore">⋯</button>
</div>
<div class="me-body">
  <div class="me-tools" id="meTools"></div>
  <div class="me-canvaswrap" id="meWrap"><canvas id="meCanvas"></canvas></div>
  <div class="me-side">
    <div class="card" id="meToolBox"></div>
    <div class="card" id="meSelBox" hidden></div>
    <div class="me-tabs" id="meTabs"></div>
    <div class="card" id="meTab"></div>
    <div class="card"><canvas id="meMini" class="me-mini"></canvas></div>
  </div>
</div>
<div class="me-status" id="meStatus"></div>
<div id="meMoreMenu" class="card" style="position:absolute; right:16px; top:100px; z-index:8; display:none; width:260px">
  <div class="stack">
    <button id="meReload">Ricarica dai file (scarta le modifiche)</button>
    <button id="meExport">Esporta in JSON…</button>
    <label class="row" style="margin:0"><button id="meImportBtn">Importa da JSON…</button><input id="meImport" type="file" accept=".json" hidden></label>
    <button id="mePng">Esporta questa mappa come immagine</button>
    <div class="muted">Scorciatoie: T mano · V B E L R O F I Q M Z P H N K strumenti · X scambia le caselle · [ ] grandezza del pennello · G griglia · 0 adatta · spazio o tasto centrale: sposta la vista · rotella: zoom · tasto destro: dipinge la secondaria · Ctrl+Z/Y · Ctrl+C/X/V nella selezione · Canc elimina · frecce: sposta di una casella · Esc annulla.</div>
  </div>
</div>`;
    const dlg = document.createElement("div");
    dlg.id = "meLegDlg"; dlg.className = "modal";
    dlg.innerHTML = `<div class="modalbox" style="width:min(1180px,100%)"><div class="row" style="justify-content:space-between"><h3 style="margin:0">Legenda: i tipi di casella</h3><button id="meLegClose">Chiudi</button></div><div id="meLegBody" style="overflow:auto"></div></div>`;
    document.body.appendChild(dlg);
    const dlg2 = document.createElement("div");
    dlg2.id = "meDlg"; dlg2.className = "modal";
    dlg2.innerHTML = `<div class="modalbox" style="width:min(640px,100%)"><div id="meDlgBody"></div></div>`;
    document.body.appendChild(dlg2);
    dlg2.addEventListener("pointerdown", (e) => { if (e.target === dlg2) dlg2.style.display = "none"; });
    $("meNewMap").onclick = newMapDialog; $("meGrow").onclick = growDialog;
    dlg.querySelector("#meLegClose").onclick = () => { dlg.style.display = "none"; st.tab = "palette"; renderTab(); };
    $("meTools").innerHTML = TOOLS.map(([id, ic, tip]) => `<button data-tool="${id}" title="${esc(tip)}">${ic}</button>`).join("");
    for (const b of $("meTools").querySelectorAll("button")) b.onclick = () => setTool(b.dataset.tool);
    $("meTabs").innerHTML = [["palette", "Tavolozza"], ["items", "Elementi"], ["zones", "Zone"], ["map", "Mappa"], ["legend", "Legenda"], ["check", "Verifica"]]
      .map(([id, t]) => `<button data-tab="${id}">${t}</button>`).join("");
    for (const b of $("meTabs").querySelectorAll("button")) b.onclick = () => { st.tab = b.dataset.tab; renderTab(); };
    for (const c of sec.querySelectorAll("[data-show]")) { c.checked = st.show[c.dataset.show]; c.onchange = () => { st.show[c.dataset.show] = c.checked; draw(); }; }
    $("meLayer").onchange = () => setLayer(Number($("meLayer").value));
    $("meUndo").onclick = () => undoRedo(false); $("meRedo").onclick = () => undoRedo(true);
    $("meFit").onclick = () => { fit(); draw(); };
    $("meSave").onclick = () => save(false); $("meSaveRestart").onclick = () => { if (confirm("Salvare e iniziare una nuova partita? La partita in corso si perde (salvala prima, se serve).")) save(true); };
    $("meCheck").onclick = () => runCheck();
    $("meLive").onclick = applyLive;
    $("meMore").onclick = () => { const m = $("meMoreMenu"); m.style.display = m.style.display === "none" ? "block" : "none"; };
    $("meReload").onclick = async () => { $("meMoreMenu").style.display = "none"; if (!st.dirty || confirm("Scartare tutte le modifiche non salvate?")) { try { localStorage.removeItem("mapEditorDraft"); } catch (e) {} load(false); } };
    $("meExport").onclick = () => { $("meMoreMenu").style.display = "none"; download("mappa.json", JSON.stringify(serialize(), null, 1), "application/json"); };
    $("meImportBtn").onclick = () => $("meImport").click();
    $("meImport").onchange = async () => {
      const f = $("meImport").files[0]; if (!f) return;
      try {
        const d = JSON.parse(await f.text());
        if (!d.map?.layers) throw new Error("manca map.layers");
        change("importa", () => { st.doc = adopt(d); rebuildLegend(); st.layer = 0; }, true);
        $("meMoreMenu").style.display = "none"; fit(); buildTerrain(); renderLayerSelect(); renderAll(); toast("Importato: salva per renderlo definitivo");
      } catch (e) { toast("File non valido: " + e.message, true); }
      $("meImport").value = "";
    };
    $("mePng").onclick = () => { $("meMoreMenu").style.display = "none"; st.terrain.toBlob((b) => download(L().id + ".png", b)); };
    const cv = $("meCanvas");
    cv.addEventListener("contextmenu", (e) => e.preventDefault());
    cv.addEventListener("wheel", onWheel, { passive: false });
    cv.addEventListener("pointerdown", onDown);
    cv.addEventListener("pointermove", onMove);
    cv.addEventListener("pointerleave", () => { st.hover = null; draw(); renderStatus(); });
    window.addEventListener("pointerup", onUp);
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", (e) => { if (e.code === "Space") { st.space = false; if (st.built && !st.pan) setCursor(); } });
    window.addEventListener("resize", () => { if (visible()) resize(); });
    window.addEventListener("beforeunload", (e) => { if (st.dirty) { e.preventDefault(); e.returnValue = ""; } });
    const mini = $("meMini");
    const miniJump = (e) => {
      const r = mini.getBoundingClientRect(), l = L();
      const x = (e.clientX - r.left) / r.width * l.width, y = (e.clientY - r.top) / r.height * l.height;
      centerOn(x, y, false);
    };
    mini.addEventListener("pointerdown", (e) => { miniJump(e); mini.setPointerCapture(e.pointerId); mini.onpointermove = (ev) => { if (ev.buttons) miniJump(ev); }; });
  }
  function download(name, data, type) {
    const blob = data instanceof Blob ? data : new Blob([data], { type });
    const a = document.createElement("a"); a.href = URL.createObjectURL(blob); a.download = name; a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 2000);
  }
  const visible = () => $("s-mapedit")?.classList.contains("on");

  // ── View ────────────────────────────────────────────────────────────────────
  function resize() {
    const cv = $("meCanvas"), r = cv.getBoundingClientRect();
    if (!r.width) return;
    cv.width = r.width * devicePixelRatio; cv.height = r.height * devicePixelRatio;
    if (!st.view.fitted && st.doc) fit();
    draw();
  }
  function fit() {
    const r = $("meCanvas").getBoundingClientRect(), l = L();
    if (!r.width || !r.height) return;
    st.view.fitted = true;
    st.view.scale = Math.max(0.05, Math.min(r.width / (l.width * TS), r.height / (l.height * TS)));
    st.view.ox = (r.width - l.width * TS * st.view.scale) / 2; st.view.oy = (r.height - l.height * TS * st.view.scale) / 2;
  }
  function centerOn(x, y, zoom = true) {
    const r = $("meCanvas").getBoundingClientRect();
    if (zoom) st.view.scale = Math.max(st.view.scale, 1.5);
    st.view.ox = r.width / 2 - (x + 0.5) * TS * st.view.scale; st.view.oy = r.height / 2 - (y + 0.5) * TS * st.view.scale;
    draw();
  }
  function setLayer(i) {
    st.layer = clampN(i, 0, st.doc.map.layers.length - 1); $("meLayer").value = String(st.layer);
    st.area = null; st.hover = null; st.drag = null; st.view.fitted = false; fit(); buildTerrain(); renderAll();
  }
  function onWheel(e) {
    e.preventDefault();
    const r = $("meCanvas").getBoundingClientRect(), mx = e.clientX - r.left, my = e.clientY - r.top, v = st.view;
    const ns = clampN(v.scale * (e.deltaY < 0 ? 1.2 : 1 / 1.2), 0.05, 10);
    v.ox = mx - (mx - v.ox) * (ns / v.scale); v.oy = my - (my - v.oy) * (ns / v.scale); v.scale = ns; draw();
  }
  function cellAt(e, clamp) {
    const r = $("meCanvas").getBoundingClientRect(), v = st.view, l = L();
    let x = Math.floor((e.clientX - r.left - v.ox) / v.scale / TS), y = Math.floor((e.clientY - r.top - v.oy) / v.scale / TS);
    if (clamp) { x = clampN(x, 0, l.width - 1); y = clampN(y, 0, l.height - 1); }
    return x >= 0 && y >= 0 && x < l.width && y < l.height ? { x, y } : null;
  }

  // ── Drawing ─────────────────────────────────────────────────────────────────
  const sprCache = {};
  function sprite(file) {
    if (!file || !S.assets.has(file)) return null;
    if (!(file in sprCache)) { sprCache[file] = null; loadImg(file).then((im) => { sprCache[file] = im; draw(); }); }
    return sprCache[file];
  }
  const bldSheet = (id) => (typeof sheetOf === "function" ? sheetOf("building:" + id) : null);
  function npcSheet(tpl) {
    const t = C("templates")[tpl]; if (!t) return null;
    const s = typeof sheetOf === "function" ? sheetOf("race:" + t.race) : null;
    return (s ?? "chibi_" + t.race) + ".png";
  }
  function itemFile(id) {
    const d = C("items")[id];
    return ["item_" + id, "item_" + d?.category, "item_varie"].find((x) => S.assets.has(x + ".png")) + ".png";
  }
  function cellsOfTool(a, b) {
    const t = st.tool, out = [];
    const x0 = Math.min(a.x, b.x), x1 = Math.max(a.x, b.x), y0 = Math.min(a.y, b.y), y1 = Math.max(a.y, b.y);
    if (t === "line") { for (const p of lineCells(a, b)) out.push(...stamp(p.x, p.y)); }
    else if (t === "rect") { for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) if (!st.outline || x === x0 || x === x1 || y === y0 || y === y1) out.push({ x, y }); }
    else if (t === "ellipse") {
      const cx = (x0 + x1) / 2, cy = (y0 + y1) / 2, rx = (x1 - x0) / 2 + 0.5, ry = (y1 - y0) / 2 + 0.5;
      const inside = (x, y) => ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1;
      for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) if (inside(x, y) && (!st.outline || !inside(x - 1, y) || !inside(x + 1, y) || !inside(x, y - 1) || !inside(x, y + 1))) out.push({ x, y });
    }
    return out;
  }
  function lineCells(a, b) {
    const out = []; let x = a.x, y = a.y; const dx = Math.abs(b.x - x), dy = -Math.abs(b.y - y), sx = x < b.x ? 1 : -1, sy = y < b.y ? 1 : -1; let err = dx + dy;
    for (;;) { out.push({ x, y }); if (x === b.x && y === b.y) break; const e2 = 2 * err; if (e2 >= dy) { err += dy; x += sx; } if (e2 <= dx) { err += dx; y += sy; } }
    return out;
  }
  function stamp(cx, cy) {
    const out = [], n = st.brush, o = Math.floor((n - 1) / 2);
    for (let dy = 0; dy < n; dy++) for (let dx = 0; dx < n; dx++) {
      if (st.round && n > 2) { const ex = dx - (n - 1) / 2, ey = dy - (n - 1) / 2; if (ex * ex + ey * ey > (n / 2) ** 2) continue; }
      out.push({ x: cx - o + dx, y: cy - o + dy });
    }
    return out;
  }
  function draw() {
    if (!st.doc || !visible()) return;
    const cv = $("meCanvas"), ctx = cv.getContext("2d"), dpr = devicePixelRatio, v = st.view, l = L();
    ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.fillStyle = "#0d0f12"; ctx.fillRect(0, 0, cv.width, cv.height);
    ctx.setTransform(dpr * v.scale, 0, 0, dpr * v.scale, dpr * v.ox, dpr * v.oy);
    ctx.imageSmoothingEnabled = false;
    if (st.terrain) ctx.drawImage(st.terrain, 0, 0);
    const px = 1 / v.scale, inArea = areaItems();
    // Visible cell range.
    const r = cv.getBoundingClientRect();
    const vx0 = clampN(Math.floor(-v.ox / v.scale / TS), 0, l.width), vy0 = clampN(Math.floor(-v.oy / v.scale / TS), 0, l.height);
    const vx1 = clampN(Math.ceil((r.width - v.ox) / v.scale / TS), 0, l.width), vy1 = clampN(Math.ceil((r.height - v.oy) / v.scale / TS), 0, l.height);
    if (st.show.solid) {
      ctx.fillStyle = "rgba(230,40,40,.38)";
      for (let y = vy0; y < vy1; y++) for (let x = vx0; x < vx1; x++) if (!walkable(l.grid[y][x])) ctx.fillRect(x * TS, y * TS, TS, TS);
    }
    if (st.show.grid && v.scale * TS >= 7) {
      ctx.strokeStyle = "rgba(0,0,0,.18)"; ctx.lineWidth = px; ctx.beginPath();
      for (let x = vx0; x <= vx1; x++) { ctx.moveTo(x * TS, vy0 * TS); ctx.lineTo(x * TS, vy1 * TS); }
      for (let y = vy0; y <= vy1; y++) { ctx.moveTo(vx0 * TS, y * TS); ctx.lineTo(vx1 * TS, y * TS); }
      ctx.stroke();
    }
    ctx.strokeStyle = "rgba(255,255,255,.5)"; ctx.lineWidth = 2 * px; ctx.strokeRect(0, 0, l.width * TS, l.height * TS);
    const label = (text, x, y, bg = "rgba(0,0,0,.6)", fg = "#fff", size = 11) => {
      if (!st.show.labels) return;
      ctx.font = `${size * px}px system-ui`; const w = ctx.measureText(text).width;
      ctx.fillStyle = bg; ctx.fillRect(x, y, w + 6 * px, (size + 4) * px); ctx.fillStyle = fg; ctx.fillText(text, x + 3 * px, y + (size + 0.5) * px);
    };
    // Borders shared with other maps.
    for (const e of st.doc.map.edges) {
      for (const [me, side, other] of [[e.a, e.side, e.b], [e.b, { North: "South", South: "North", East: "West", West: "East" }[e.side], e.a]]) {
        if (me !== l.id) continue;
        ctx.fillStyle = "rgba(255,200,40,.75)";
        const t = 4 * px;
        if (side === "North") ctx.fillRect(0, -t, l.width * TS, t); if (side === "South") ctx.fillRect(0, l.height * TS, l.width * TS, t);
        if (side === "West") ctx.fillRect(-t, 0, t, l.height * TS); if (side === "East") ctx.fillRect(l.width * TS, 0, t, l.height * TS);
        const name = "→ " + (st.doc.map.layers[layerIdx(other)]?.name ?? other);
        const pos = { North: [l.width * TS / 2, -18 * px], South: [l.width * TS / 2, l.height * TS + 6 * px], West: [-90 * px, l.height * TS / 2], East: [l.width * TS + 8 * px, l.height * TS / 2] }[side];
        label(name, pos[0], pos[1], "rgba(120,90,0,.85)");
      }
    }
    if (st.show.zones) {
      for (const z of zones()) {
        if (z.layer !== l.id) continue;
        const [x, y, w, h] = z.rect, on = (st.sel?.type === "zone" && st.sel.id === z.id) || inArea.zones.includes(z.id);
        ctx.strokeStyle = on ? "#ffeb3b" : (z.tags ?? []).includes("quartiere") ? "rgba(255,255,255,.75)" : "rgba(140,220,255,.75)";
        ctx.lineWidth = (on ? 3 : 1.5) * px; ctx.setLineDash(on ? [] : [6 * px, 3 * px]); ctx.strokeRect(x * TS, y * TS, w * TS, h * TS); ctx.setLineDash([]);
        if (on) { ctx.fillStyle = "rgba(255,235,59,.12)"; ctx.fillRect(x * TS, y * TS, w * TS, h * TS); for (const [hx, hy] of corners(z)) { ctx.fillStyle = "#ffeb3b"; ctx.fillRect(hx * TS - 4 * px, hy * TS - 4 * px, 8 * px, 8 * px); } }
        if (on || w * TS * v.scale > 70) label(z.name, x * TS + 2 * px, y * TS + 2 * px);
      }
    }
    if (st.show.portals) {
      for (const [i, p] of st.doc.map.portals.entries()) for (const [end, o] of [[p.a, p.b], [p.b, p.a]]) {
        if (end[0] !== l.id) continue;
        const on = (st.sel?.type === "portal" && st.sel.i === i) || inArea.portals.includes(i);
        const cx = (end[1] + 0.5) * TS, cy = (end[2] + 0.5) * TS;
        ctx.fillStyle = on ? "#ffeb3b" : "#b46cff"; ctx.beginPath(); ctx.arc(cx, cy, Math.max(6, 4 * px), 0, 7); ctx.fill();
        ctx.strokeStyle = "#000"; ctx.lineWidth = px; ctx.stroke();
        ctx.fillStyle = "#fff"; ctx.font = `bold ${Math.max(9, 8 * px)}px system-ui`; ctx.textAlign = "center"; ctx.fillText("⇅", cx, cy + Math.max(3, 3 * px)); ctx.textAlign = "start";
        if (on || v.scale > 0.45) label(`${p.name} → ${st.doc.map.layers[layerIdx(o[0])]?.name ?? o[0]}`, cx + 8 * px, cy - 6 * px, "rgba(80,30,120,.85)");
      }
      if (st.pendingPortal && st.pendingPortal.layer === st.layer) { ctx.strokeStyle = "#b46cff"; ctx.lineWidth = 3 * px; ctx.strokeRect(st.pendingPortal.x * TS, st.pendingPortal.y * TS, TS, TS); }
    }
    const drawPl = (p, i, other) => {
      const P = pl(p);
      if (!P.pos || P.pos.layer !== st.layer) return;
      const show = P.kind === "Hidden" ? st.show.hidden : P.kind === "Building" ? st.show.buildings : st.show.npcs;
      if (!show) return;
      const on = !other && ((st.sel?.type === "pl" && st.sel.i === i) || inArea.pls.includes(i));
      const { x, y } = P.pos;
      ctx.globalAlpha = other ? 0.55 : 1;
      if (P.kind === "Building") {
        const d = C("buildings")[P.b.building], [fw, fh] = d?.footprint ?? [1, 1];
        const im = sprite((bldSheet(P.b.building) ?? "") + ".png");
        const x0 = (x - Math.floor((fw - 1) / 2)) * TS, y0 = (y - fh + 1) * TS;
        ctx.fillStyle = "rgba(255,255,255,.14)"; ctx.fillRect(x0, y0, fw * TS, fh * TS);
        if (im) { const w = Math.max(fw, 1) * TS, h = im.naturalHeight * (w / im.naturalWidth); ctx.drawImage(im, x0, (y + 1) * TS - h, w, h); }
        ctx.strokeStyle = on ? "#ffeb3b" : P.b.hidden ? "#ff8a3d" : P.b.owner_faction ? hashColor(P.b.owner_faction) : "#ddd";
        ctx.lineWidth = (on ? 3 : 1.5) * px; if (P.b.hidden) ctx.setLineDash([4 * px, 3 * px]); ctx.strokeRect(x0, y0, fw * TS, fh * TS); ctx.setLineDash([]);
        ctx.strokeStyle = "#fff"; ctx.strokeRect(x * TS + 4, y * TS + 4, TS - 8, TS - 8);
        if (v.scale > 0.6) label((P.b.hidden ? "🔒 " : "") + (P.b.name || d?.name || P.b.building), x0, (y + 1) * TS + 1 * px, "rgba(0,0,0,.65)", "#fff", 10);
      } else if (P.kind === "Hidden") {
        const it = Object.keys(P.b.stock ?? {})[0], im = it ? sprite(itemFile(it)) : null;
        ctx.fillStyle = "rgba(255,138,61,.3)"; ctx.fillRect(x * TS, y * TS, TS, TS);
        if (im) ctx.drawImage(im, x * TS, y * TS, TS, TS); else { ctx.fillStyle = "#ff8a3d"; ctx.font = `${TS}px system-ui`; ctx.fillText("✦", x * TS, (y + 0.9) * TS); }
        ctx.strokeStyle = on ? "#ffeb3b" : "#ff8a3d"; ctx.lineWidth = (on ? 3 : 1.5) * px; ctx.setLineDash([3 * px, 2 * px]); ctx.strokeRect(x * TS, y * TS, TS, TS); ctx.setLineDash([]);
        if (v.scale > 1) label(Object.entries(P.b.stock ?? {}).map(([k, q]) => `${q}× ${C("items")[k]?.name ?? k}`).join(", ") || "vuoto", (x + 1) * TS, y * TS, "rgba(120,50,0,.85)", "#fff", 9);
      } else if (P.kind === "Entity") {
        const im = sprite(npcSheet(P.b.template));
        const n = P.b.count ?? 1;
        if (im) ctx.drawImage(im, 0, 0, 24, 32, x * TS - 4, y * TS - 14, 24, 32);
        else { ctx.fillStyle = hashColor(P.b.faction ?? P.b.template); ctx.beginPath(); ctx.arc((x + 0.5) * TS, (y + 0.5) * TS, 6, 0, 7); ctx.fill(); }
        if (on) { ctx.strokeStyle = "#ffeb3b"; ctx.lineWidth = 3 * px; ctx.strokeRect(x * TS - 4, y * TS - 14, 24, 32); }
        if (P.random) { ctx.strokeStyle = "rgba(255,255,255,.6)"; ctx.setLineDash([3 * px, 3 * px]); const z = zoneInfo(P.b.zone); ctx.strokeRect(z.x * TS, z.y * TS, z.w * TS, z.h * TS); ctx.setLineDash([]); }
        if (v.scale > 0.8) label((P.b.name || C("templates")[P.b.template]?.name || P.b.template) + (n > 1 ? ` ×${n}` : "") + (P.random ? " (a caso)" : ""), x * TS - 4, (y + 1) * TS + 2 * px, "rgba(0,0,0,.65)", "#fff", 10);
      }
      ctx.globalAlpha = 1;
    };
    if (st.show.others) for (const o of st.others) drawPl(o.placement, -1, true);
    st.doc.placements.forEach((p, i) => drawPl(p, i, false));
    // Tool preview.
    if (st.drag && ["line", "rect", "ellipse"].includes(st.tool)) {
      ctx.globalAlpha = 0.6;
      const t = st.leg[st.drag.ch];
      for (const c of cellsOfTool(st.drag.a, st.drag.b)) ctx.drawImage(art(t), c.x * TS, c.y * TS);
      ctx.globalAlpha = 1;
    }
    if (st.drag && ["room", "zone", "area"].includes(st.tool)) {
      const a = st.drag.a, b = st.drag.b, x0 = Math.min(a.x, b.x), y0 = Math.min(a.y, b.y), w = Math.abs(a.x - b.x) + 1, h = Math.abs(a.y - b.y) + 1;
      ctx.strokeStyle = st.tool === "zone" ? "#8cdcff" : "#ffeb3b"; ctx.lineWidth = 2 * px; ctx.setLineDash([5 * px, 3 * px]); ctx.strokeRect(x0 * TS, y0 * TS, w * TS, h * TS); ctx.setLineDash([]);
      label(`${w}×${h}`, (x0 + w) * TS + 3 * px, (y0 + h) * TS);
    }
    if (st.area) {
      const a = st.area;
      ctx.strokeStyle = "#fff"; ctx.lineWidth = 2 * px; ctx.setLineDash([5 * px, 4 * px]); ctx.strokeRect(a.x * TS, a.y * TS, a.w * TS, a.h * TS);
      ctx.strokeStyle = "#000"; ctx.lineDashOffset = 5 * px; ctx.strokeRect(a.x * TS, a.y * TS, a.w * TS, a.h * TS); ctx.setLineDash([]); ctx.lineDashOffset = 0;
    }
    if (st.hover) {
      const h = st.hover;
      if (st.pasting && st.clip) {
        ctx.globalAlpha = 0.65;
        st.clip.cells?.forEach((row, dy) => row.forEach((ch, dx) => ctx.drawImage(art(st.leg[ch]), (h.x + dx) * TS, (h.y + dy) * TS)));
        ctx.fillStyle = "#ffeb3b"; for (const p of st.clip.items ?? []) { const b = Object.values(p)[0]; ctx.beginPath(); ctx.arc((h.x + b.at[0] + 0.5) * TS, (h.y + b.at[1] + 0.5) * TS, 5, 0, 7); ctx.fill(); }
        ctx.globalAlpha = 1; ctx.strokeStyle = "#ffeb3b"; ctx.lineWidth = 2 * px; ctx.strokeRect(h.x * TS, h.y * TS, st.clip.w * TS, st.clip.h * TS);
      } else if (["brush", "eraser"].includes(st.tool) || (["line", "rect", "ellipse", "fill"].includes(st.tool) && !st.drag)) {
        ctx.strokeStyle = "#ffeb3b"; ctx.lineWidth = 1.5 * px;
        for (const c of (["brush", "eraser", "line"].includes(st.tool) ? stamp(h.x, h.y) : [h])) ctx.strokeRect(c.x * TS, c.y * TS, TS, TS);
      } else if (st.tool !== "hand") { ctx.strokeStyle = "#ffeb3b"; ctx.lineWidth = 2 * px; ctx.strokeRect(h.x * TS, h.y * TS, TS, TS); }
    }
    $("meZoom").textContent = Math.round(v.scale * 100) + "%";
    drawMiniView();
  }
  function corners(z) { const [x, y, w, h] = z.rect; return [[x, y], [x + w, y], [x, y + h], [x + w, y + h]]; }
  let miniBase = null;
  function drawMini() {
    if (!st.terrain) return;
    const mini = $("meMini"), l = L(), w = 300, h = Math.max(40, Math.round(w * l.height / l.width));
    mini.width = w; mini.height = h;
    miniBase = document.createElement("canvas"); miniBase.width = w; miniBase.height = h;
    const c = miniBase.getContext("2d"); c.imageSmoothingEnabled = true; c.drawImage(st.terrain, 0, 0, w, h);
    drawMiniView();
  }
  function drawMiniView() {
    const mini = $("meMini"); if (!miniBase || !mini) return;
    const c = mini.getContext("2d"), l = L(), v = st.view, r = $("meCanvas").getBoundingClientRect();
    c.drawImage(miniBase, 0, 0);
    const k = mini.width / (l.width * TS);
    c.strokeStyle = "#ffeb3b"; c.lineWidth = 2;
    c.strokeRect(-v.ox / v.scale * k, -v.oy / v.scale * k, r.width / v.scale * k, r.height / v.scale * k);
  }

  // ── Pointer ─────────────────────────────────────────────────────────────────
  function hit(c) {
    const cand = [];
    st.doc.placements.forEach((p, i) => {
      const P = pl(p); if (!P.pos || P.pos.layer !== st.layer) return;
      const show = P.kind === "Hidden" ? st.show.hidden : P.kind === "Building" ? st.show.buildings : st.show.npcs; if (!show) return;
      let inside = P.pos.x === c.x && P.pos.y === c.y, d = 0;
      if (!inside && P.kind === "Building") {
        const [fw, fh] = C("buildings")[P.b.building]?.footprint ?? [1, 1];
        inside = c.x >= P.pos.x - Math.floor((fw - 1) / 2) && c.x <= P.pos.x + Math.floor(fw / 2) && c.y <= P.pos.y && c.y > P.pos.y - fh; d = 2;
      }
      if (!inside && P.kind === "Entity" && c.x === P.pos.x && c.y === P.pos.y - 1) { inside = true; d = 1; }
      if (inside) cand.push({ type: "pl", i, d });
    });
    if (st.show.portals) st.doc.map.portals.forEach((p, i) => { for (const [k, e] of [["a", p.a], ["b", p.b]]) if (e[0] === L().id && e[1] === c.x && e[2] === c.y) cand.push({ type: "portal", i, end: k, d: 0 }); });
    cand.sort((a, b) => a.d - b.d);
    if (cand.length) return cand[0];
    if (st.show.others) {
      const o = st.others.find((o) => { const P = pl(o.placement); return P.pos && P.pos.layer === st.layer && P.pos.x === c.x && P.pos.y === c.y; });
      if (o) return { type: "other", o };
    }
    if (st.show.zones) {
      const z = zones().filter((z) => z.layer === L().id && c.x >= z.rect[0] && c.y >= z.rect[1] && c.x < z.rect[0] + z.rect[2] && c.y < z.rect[1] + z.rect[3])
        .sort((a, b) => a.rect[2] * a.rect[3] - b.rect[2] * b.rect[3])[0];
      if (z) return { type: "zone", id: z.id };
    }
    return null;
  }
  function onDown(e) {
    if (!st.doc) return;
    $("meMoreMenu").style.display = "none";
    const cv = $("meCanvas");
    if (e.button === 1 || (e.button === 0 && st.space) || st.tool === "hand") { cv.style.cursor = "grabbing"; st.pan = { x: e.clientX, y: e.clientY, ox: st.view.ox, oy: st.view.oy }; cv.setPointerCapture(e.pointerId); return; }
    const c = cellAt(e, false); if (!c) return;
    cv.setPointerCapture(e.pointerId);
    const right = e.button === 2, ch = right ? st.secondary : st.primary;
    if (st.pasting) { if (right) { st.pasting = false; draw(); return; } pasteAt(c); return; }
    switch (st.tool) {
      case "brush": case "eraser": {
        const t = st.tool === "eraser" ? (right ? st.primary : st.secondary) : ch;
        beginTiles(); st.drag = { ch: t, last: c }; for (const p of stamp(c.x, c.y)) setTile(p.x, p.y, t); draw(); break;
      }
      case "line": case "rect": case "ellipse": st.drag = { ch, a: c, b: c }; draw(); break;
      case "room": case "zone": case "area": if (!right) { st.drag = { a: c, b: c }; if (st.tool === "area") st.area = null; draw(); } else { st.area = null; draw(); } break;
      case "fill": floodFill(c, ch); break;
      case "pick": { const t = getTile(c.x, c.y); if (right) st.secondary = t; else st.primary = t; toast("Presa: " + tileName(t)); renderToolBox(); if (st.tab === "palette") renderTab(); break; }
      case "portal": portalClick(c); break;
      case "building": case "npc": case "hidden": if (!right) placeAt(c); break;
      case "select": {
        const h = hit(c);
        if (h?.type === "other") { st.sel = h; renderSel(); draw(); break; }
        // A selected zone's corner handle resizes it.
        if (st.sel?.type === "zone") {
          const z = zones().find((z) => z.id === st.sel.id);
          const k = z && corners(z).findIndex(([x, y]) => Math.abs(x - (c.x + 0.5)) <= 0.9 && Math.abs(y - (c.y + 0.5)) <= 0.9);
          if (z && k >= 0) { st.drag = { kind: "resize", corner: k, before: snap(false), rect0: [...z.rect] }; break; }
        }
        st.sel = h; renderSel(); draw();
        if (h && !right) st.drag = { kind: "move", start: c, before: snap(false), moved: false };
        break;
      }
    }
  }
  function onMove(e) {
    if (!st.doc) return;
    if (st.pan) { st.view.ox = st.pan.ox + e.clientX - st.pan.x; st.view.oy = st.pan.oy + e.clientY - st.pan.y; draw(); return; }
    const c = cellAt(e, !!st.drag);
    st.hover = c;
    if (st.drag && c) {
      const d = st.drag;
      if (["brush", "eraser"].includes(st.tool) && d.last) { for (const p of lineCells(d.last, c)) for (const q of stamp(p.x, p.y)) setTile(q.x, q.y, d.ch); d.last = c; }
      else if (["line", "rect", "ellipse", "room", "zone", "area"].includes(st.tool) && d.a) {
        let b = c;
        if (e.shiftKey && st.tool === "line") { const dx = Math.abs(c.x - d.a.x), dy = Math.abs(c.y - d.a.y); b = dx > dy * 2 ? { x: c.x, y: d.a.y } : dy > dx * 2 ? { x: d.a.x, y: c.y } : { x: d.a.x + Math.sign(c.x - d.a.x) * Math.max(dx, dy), y: d.a.y + Math.sign(c.y - d.a.y) * Math.max(dx, dy) }; }
        else if (e.shiftKey) { const m = Math.max(Math.abs(c.x - d.a.x), Math.abs(c.y - d.a.y)); b = { x: d.a.x + Math.sign(c.x - d.a.x || 1) * m, y: d.a.y + Math.sign(c.y - d.a.y || 1) * m }; }
        d.b = b;
      } else if (d.kind === "move" && (c.x !== d.start.x || c.y !== d.start.y)) { moveSel(c.x - d.start.x, c.y - d.start.y); d.start = c; d.moved = true; }
      else if (d.kind === "resize") {
        const z = zones().find((z) => z.id === st.sel.id), [x0, y0, w0, h0] = d.rect0;
        let L0 = x0, T0 = y0, R0 = x0 + w0, B0 = y0 + h0;
        if (d.corner === 0 || d.corner === 2) L0 = Math.min(c.x, R0 - 1); else R0 = Math.max(c.x + 1, L0 + 1);
        if (d.corner === 0 || d.corner === 1) T0 = Math.min(c.y, B0 - 1); else B0 = Math.max(c.y + 1, T0 + 1);
        z.rect = [L0, T0, R0 - L0, B0 - T0]; d.moved = true; renderSel();
      }
    }
    draw(); renderStatus();
  }
  function onUp() {
    if (st.pan) { st.pan = null; setCursor(); return; }
    const d = st.drag; st.drag = null;
    if (!d) return;
    if (["brush", "eraser"].includes(st.tool)) endTiles(st.tool === "eraser" ? "gomma" : "pennello");
    else if (["line", "rect", "ellipse"].includes(st.tool) && d.a) {
      beginTiles(); for (const c of cellsOfTool(d.a, d.b)) setTile(c.x, c.y, d.ch); endTiles({ line: "linea", rect: "rettangolo", ellipse: "ellisse" }[st.tool]);
    } else if (st.tool === "room" && d.a) room(d.a, d.b);
    else if (st.tool === "zone" && d.a) newZone(d.a, d.b);
    else if (st.tool === "area" && d.a) {
      const x = Math.min(d.a.x, d.b.x), y = Math.min(d.a.y, d.b.y);
      st.area = { x, y, w: Math.abs(d.a.x - d.b.x) + 1, h: Math.abs(d.a.y - d.b.y) + 1 }; renderToolBox();
    } else if ((d.kind === "move" || d.kind === "resize") && d.moved) {
      if (d.kind === "move" && st.sel?.type === "pl") fixZone(st.doc.placements[st.sel.i]);
      st.undo.push({ kind: "doc", label: d.kind === "move" ? "sposta" : "ridimensiona la zona", before: d.before, after: snap(false) }); st.redo = []; touched(); renderSel(); if (st.tab !== "palette") renderTab();
    }
    draw();
  }
  /** A placement moved out of its zone gets the smallest zone it is now in. */
  function fixZone(p) {
    const P = pl(p);
    if (P.b.at && !inZone(P.b.zone, st.layer, P.b.at[0], P.b.at[1])) P.b.zone = zoneAt(P.b.at[0], P.b.at[1]);
  }
  function moveSel(dx, dy) {
    const s = st.sel, l = L();
    if (s?.type === "pl") {
      const P = pl(st.doc.placements[s.i]);
      if (!P.pos) return;
      P.b.at = [clampN(P.pos.x + dx, 0, l.width - 1), clampN(P.pos.y + dy, 0, l.height - 1)];
    } else if (s?.type === "portal") {
      const e = st.doc.map.portals[s.i][s.end]; e[1] = clampN(e[1] + dx, 0, l.width - 1); e[2] = clampN(e[2] + dy, 0, l.height - 1);
    } else if (s?.type === "zone") {
      const z = zones().find((z) => z.id === s.id); z.rect[0] += dx; z.rect[1] += dy;
    }
    renderSel();
  }

  // ── Tile operations ─────────────────────────────────────────────────────────
  function floodFill(c, ch) {
    const l = L(), from = l.grid[c.y][c.x];
    if (from === ch) return;
    beginTiles();
    if (st.fillAll) { for (let y = 0; y < l.height; y++) for (let x = 0; x < l.width; x++) if (l.grid[y][x] === from) setTile(x, y, ch); }
    else {
      const stack = [[c.x, c.y]], seen = new Uint8Array(l.width * l.height);
      while (stack.length) {
        const [x, y] = stack.pop();
        if (x < 0 || y < 0 || x >= l.width || y >= l.height || seen[y * l.width + x] || l.grid[y][x] !== from) continue;
        seen[y * l.width + x] = 1; setTile(x, y, ch);
        stack.push([x + 1, y], [x - 1, y], [x, y + 1], [x, y - 1]);
      }
    }
    endTiles(st.fillAll ? "sostituisci" : "riempi"); draw(); drawMini();
  }
  function room(a, b) {
    const x0 = Math.min(a.x, b.x), x1 = Math.max(a.x, b.x), y0 = Math.min(a.y, b.y), y1 = Math.max(a.y, b.y);
    if (x1 - x0 < 2 || y1 - y0 < 2) { toast("Una stanza deve essere almeno 3×3", true); return; }
    const o = st.opt;
    beginTiles();
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) setTile(x, y, x === x0 || x === x1 || y === y0 || y === y1 ? o.wall : o.floor);
    setTile(Math.floor((x0 + x1) / 2), y1, o.door);
    endTiles("stanza"); drawMini();
  }
  function areaCells(a) { return Array.from({ length: a.h }, (_, dy) => Array.from({ length: a.w }, (_, dx) => getTile(a.x + dx, a.y + dy))); }
  const AREA_KINDS = [["tiles", "caselle"], ["Building", "edifici"], ["Entity", "NPC"], ["Hidden", "nascosti"], ["zone", "zone"], ["portal", "passaggi"]];
  /** What the selected area holds of the kinds ticked in the tool box: placements and passages with a
   *  point inside it (NPCs placed at random in a zone are drawn, and taken, at the zone's centre), zones
   *  entirely inside it. */
  function areaItems(all) {
    const a = st.area, out = { pls: [], zones: [], portals: [], n: 0 };
    if (!a) return out;
    const k = st.areaKinds, lid = L().id, inside = (x, y) => x >= a.x && y >= a.y && x < a.x + a.w && y < a.y + a.h;
    st.doc.placements.forEach((p, i) => { const P = pl(p); if (P.pos && P.pos.layer === st.layer && (all || k[P.kind]) && inside(P.pos.x, P.pos.y)) out.pls.push(i); });
    if (all || k.zone) for (const z of zones()) if (z.layer === lid && inside(z.rect[0], z.rect[1]) && inside(z.rect[0] + z.rect[2] - 1, z.rect[1] + z.rect[3] - 1)) out.zones.push(z.id);
    if (all || k.portal) st.doc.map.portals.forEach((p, i) => { if ([p.a, p.b].some((e) => e[0] === lid && inside(e[1], e[2]))) out.portals.push(i); });
    out.n = out.pls.length + out.zones.length + out.portals.length;
    return out;
  }
  function areaSummary(it) {
    const c = { Building: 0, Entity: 0, Hidden: 0 };
    for (const i of it.pls) c[pl(st.doc.placements[i]).kind]++;
    return [[c.Building, "edifici"], [c.Entity, "NPC"], [c.Hidden, "nascosti"], [it.zones.length, "zone"], [it.portals.length, "passaggi"]].filter(([n]) => n).map(([n, t]) => `${n} ${t}`).join(", ");
  }
  /** Removes the elements of the area (inside a `change`). */
  function removeItems(it) {
    const li = st.layer, lid = L().id, gone = new Set(it.pls);
    st.doc.placements = st.doc.placements.filter((_, i) => !gone.has(i));
    const pg = new Set(it.portals);
    st.doc.map.portals = st.doc.map.portals.filter((_, i) => !pg.has(i));
    if (it.zones.length) {
      st.doc.map.zones = zones().filter((z) => !it.zones.includes(z.id));
      for (const p of st.doc.placements) { const b = Object.values(p)[0]; if (it.zones.includes(b.zone)) b.zone = b.at ? zoneAt(b.at[0], b.at[1], li) : lid; if (b.tether && it.zones.includes(b.tether.zone)) delete b.tether; }
      for (const n of st.doc.map.networks) n.zones = n.zones.filter((z) => !it.zones.includes(z));
    }
  }
  function deleteAreaItems() {
    const it = areaItems();
    if (!it.n) { toast("Nessun elemento nell'area (controlla cosa è spuntato)", true); return false; }
    const text = areaSummary(it);
    if (it.n > 5 && !confirm(`Eliminare ${text}?`)) return false;
    change(`elimina ${it.n} elementi`, () => { removeItems(it); });
    st.sel = null; toast(`Eliminati: ${text} (Ctrl+Z per annullare)`); renderSel(); renderToolBox(); draw(); if (st.tab !== "palette") renderTab();
    return true;
  }
  function moveArea(dx, dy) {
    const l = L(), a = st.area, it = areaItems();
    if (a.x + dx < 0 || a.y + dy < 0 || a.x + a.w + dx > l.width || a.y + a.h + dy > l.height) return;
    if (it.n) {
      const before = snap(false);
      for (const i of it.pls) { const b = Object.values(st.doc.placements[i])[0]; if (b.at) b.at = [b.at[0] + dx, b.at[1] + dy]; }
      for (const id of it.zones) { const z = zones().find((z) => z.id === id); z.rect[0] += dx; z.rect[1] += dy; }
      for (const i of it.portals) for (const e of [st.doc.map.portals[i].a, st.doc.map.portals[i].b]) if (e[0] === l.id && e[1] >= a.x && e[2] >= a.y && e[1] < a.x + a.w && e[2] < a.y + a.h) { e[1] += dx; e[2] += dy; }
      for (const i of it.pls) fixZone(st.doc.placements[i]);
      const last = st.undo[st.undo.length - 1];
      if (last?.areaMove) last.after = snap(false); else st.undo.push({ kind: "doc", label: `sposta ${it.n} elementi`, before, after: snap(false), areaMove: true });
      st.redo = []; touched();
    }
    a.x += dx; a.y += dy; renderToolBox(); draw();
  }
  function copyArea(cut) {
    if (!st.area) { toast("Prima seleziona un'area con lo strumento Selezione (M)", true); return; }
    const a = st.area, k = st.areaKinds, it = areaItems();
    const items = it.pls.filter((i) => Object.values(st.doc.placements[i])[0].at).map((i) => { const c = JSON.parse(JSON.stringify(st.doc.placements[i])), b = Object.values(c)[0]; b.at = [b.at[0] - a.x, b.at[1] - a.y]; return c; });
    st.clip = { w: a.w, h: a.h, cells: k.tiles ? areaCells(a) : null, items };
    if (cut) {
      if (k.tiles) { beginTiles(); forArea((x, y) => setTile(x, y, st.secondary)); endTiles("taglia"); drawMini(); }
      const cutPls = it.pls.filter((i) => Object.values(st.doc.placements[i])[0].at);
      if (cutPls.length) change(`taglia ${cutPls.length} elementi`, () => { removeItems({ pls: cutPls, zones: [], portals: [] }); });
    }
    const what = [k.tiles ? `${a.w}×${a.h} caselle` : "", items.length ? `${items.length} elementi` : ""].filter(Boolean).join(" e ") || "niente";
    toast(`${cut ? "Tagliato" : "Copiato"}: ${what}. Ctrl+V per incollare`); renderToolBox(); draw();
  }
  function forArea(f) { const a = st.area; for (let y = a.y; y < a.y + a.h; y++) for (let x = a.x; x < a.x + a.w; x++) f(x, y); }
  function pasteAt(c) {
    const l = L();
    if (st.clip.cells) { beginTiles(); st.clip.cells.forEach((row, dy) => row.forEach((ch, dx) => setTile(c.x + dx, c.y + dy, ch))); endTiles("incolla"); }
    const items = (st.clip.items ?? []).filter((p) => { const b = Object.values(p)[0]; return c.x + b.at[0] < l.width && c.y + b.at[1] < l.height; });
    if (items.length) change(`incolla ${items.length} elementi`, () => {
      for (const p of items) { const n = JSON.parse(JSON.stringify(p)), b = Object.values(n)[0]; b.at = [c.x + b.at[0], c.y + b.at[1]]; b.zone = zoneAt(b.at[0], b.at[1]); delete b.leader_of; st.doc.placements.push(n); }
    });
    draw(); drawMini(); if (st.tab === "items") renderTab();
  }
  function transformClip(kind) {
    if (!st.clip) return;
    const c = st.clip.cells, { w, h } = st.clip;
    for (const p of st.clip.items ?? []) { const b = Object.values(p)[0], [x, y] = b.at; b.at = kind === "h" ? [w - 1 - x, y] : kind === "v" ? [x, h - 1 - y] : [h - 1 - y, x]; }
    if (kind === "h") c?.forEach((r) => r.reverse());
    else if (kind === "v") c?.reverse();
    else st.clip = { w: h, h: w, items: st.clip.items, cells: c && Array.from({ length: w }, (_, y) => Array.from({ length: h }, (_, x) => c[h - 1 - x][y])) };
    draw();
  }

  // ── Placing things ──────────────────────────────────────────────────────────
  function placeAt(c) {
    const o = st.opt, zone = zoneAt(c.x, c.y);
    let p;
    if (st.tool === "building") {
      if (!C("buildings")[o.building]) { toast("Scegli un edificio", true); return; }
      p = { Building: { building: o.building, zone, at: [c.x, c.y], ...(o.bname ? { name: o.bname } : {}), ...(o.owner ? { owner_faction: o.owner } : {}) } };
    } else if (st.tool === "npc") {
      if (!C("templates")[o.template]) { toast("Scegli un template", true); return; }
      p = { Entity: { template: o.template, zone, at: [c.x, c.y], count: Math.max(1, Number(o.count) || 1), ...(o.npcName ? { name: o.npcName } : {}), ...(o.faction ? { faction: o.faction } : {}) } };
    } else {
      if (!C("items")[o.hideItem]) { toast("Scegli l'oggetto da nascondere", true); return; }
      if (!C("buildings").nascondiglio) { toast("Manca l'edificio 'nascondiglio' nei contenuti", true); return; }
      p = { Building: { building: "nascondiglio", zone, at: [c.x, c.y], hidden: true, stock: { [o.hideItem]: Math.max(1, Number(o.hideQty) || 1) } } };
    }
    if (!walkable(getTile(c.x, c.y))) toast("Attenzione: la casella non è calpestabile", true);
    change("piazza " + Object.keys(p)[0], () => { st.doc.placements.push(p); st.sel = { type: "pl", i: st.doc.placements.length - 1 }; });
    renderSel(); draw(); if (st.tab === "items") renderTab();
  }
  function portalClick(c) {
    const here = [L().id, c.x, c.y];
    if (!st.pendingPortal) {
      st.pendingPortal = { layer: st.layer, x: c.x, y: c.y };
      toast("Primo capo scelto: ora clicca l'altro capo (anche su un'altra mappa, cambiandola in alto)"); renderToolBox(); draw(); return;
    }
    const a = st.pendingPortal; st.pendingPortal = null;
    if (a.layer === st.layer && a.x === c.x && a.y === c.y) { renderToolBox(); draw(); return; }
    const name = prompt("Nome del passaggio (es. «Scala della cantina»):", "Passaggio") || "Passaggio";
    change("nuovo passaggio", () => { st.doc.map.portals.push({ name, a: [st.doc.map.layers[a.layer].id, a.x, a.y], b: here }); st.sel = { type: "portal", i: st.doc.map.portals.length - 1, end: "b" }; });
    renderToolBox(); renderSel(); draw();
  }
  function newZone(a, b) {
    const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y), w = Math.abs(a.x - b.x) + 1, h = Math.abs(a.y - b.y) + 1;
    if (w * h < 2) return;
    const name = prompt("Nome della zona:", "Nuova zona"); if (!name) return;
    let id = slug(name), n = 2; while (zones().some((z) => z.id === id) || layerIdx(id) >= 0) id = slug(name) + "_" + n++;
    change("nuova zona", () => { zones().push({ id, name, layer: L().id, rect: [x, y, w, h], tags: [] }); st.sel = { type: "zone", id }; });
    renderSel(); draw(); if (st.tab === "zones") renderTab();
  }
  const slug = (s) => String(s).toLowerCase().normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "") || "x";
  function deleteSel() {
    const s = st.sel; if (!s) return;
    if (s.type === "pl") change("elimina", () => { st.doc.placements.splice(s.i, 1); });
    else if (s.type === "portal") change("elimina il passaggio", () => { st.doc.map.portals.splice(s.i, 1); });
    else if (s.type === "zone") {
      const users = st.doc.placements.filter((p) => pl(p).b.zone === s.id).length + st.others.filter((o) => pl(o.placement).b.zone === s.id).length;
      if (users && !confirm(`${users} piazzamenti usano questa zona: verranno spostati nella zona che li contiene. Eliminarla?`)) return;
      change("elimina la zona", () => {
        const z = zones().find((z) => z.id === s.id), li = layerIdx(z.layer);
        st.doc.map.zones = zones().filter((x) => x.id !== s.id);
        for (const p of st.doc.placements) { const b = pl(p).b; if (b.zone === s.id) { b.zone = b.at ? zoneAt(b.at[0], b.at[1], li) : z.layer; } }
        for (const n of st.doc.map.networks) n.zones = n.zones.filter((x) => x !== s.id);
      });
    } else return;
    st.sel = null; renderSel(); draw(); renderTab();
  }

  // ── Keyboard ────────────────────────────────────────────────────────────────
  function onKey(e) {
    if (!visible() || !st.doc) return;
    const tag = document.activeElement?.tagName;
    if (["INPUT", "TEXTAREA", "SELECT"].includes(tag)) return;
    const k = e.key.toLowerCase(), mod = e.ctrlKey || e.metaKey;
    if (e.code === "Space") { st.space = true; setCursor(); e.preventDefault(); return; }
    if (mod && k === "z") { e.preventDefault(); undoRedo(e.shiftKey); return; }
    if (mod && k === "y") { e.preventDefault(); undoRedo(true); return; }
    if (mod && k === "s") { e.preventDefault(); save(false); return; }
    if (mod && k === "c") { e.preventDefault(); copyArea(false); return; }
    if (mod && k === "x") { e.preventDefault(); copyArea(true); return; }
    if (mod && k === "v") { e.preventDefault(); if (st.clip) { st.pasting = true; toast("Clic per incollare, R ruota, Esc per finire"); draw(); } return; }
    if (mod) return;
    if (k === "escape") { st.pasting = false; st.pendingPortal = null; st.drag = null; st.area = null; st.sel = null; renderSel(); renderToolBox(); draw(); return; }
    if (k === "delete" || k === "backspace") {
      if (st.area && !st.sel) {
        // Elements first; an area without any gets its tiles cleared.
        if (areaItems().n) deleteAreaItems();
        else if (st.areaKinds.tiles) { beginTiles(); forArea((x, y) => setTile(x, y, st.secondary)); endTiles("svuota l'area"); draw(); drawMini(); }
      } else deleteSel();
      e.preventDefault(); return;
    }
    if (st.pasting && k === "r") { transformClip("r"); return; }
    if (k.startsWith("arrow") && st.area && !st.sel) { e.preventDefault(); moveArea(k === "arrowleft" ? -1 : k === "arrowright" ? 1 : 0, k === "arrowup" ? -1 : k === "arrowdown" ? 1 : 0); return; }
    if (k.startsWith("arrow") && st.sel && st.sel.type !== "other") {
      e.preventDefault();
      const before = snap(false);
      moveSel(k === "arrowleft" ? -1 : k === "arrowright" ? 1 : 0, k === "arrowup" ? -1 : k === "arrowdown" ? 1 : 0);
      if (st.sel.type === "pl") fixZone(st.doc.placements[st.sel.i]);
      st.undo.push({ kind: "doc", label: "sposta", before, after: snap(false) }); st.redo = []; touched(); draw(); return;
    }
    if (k === "x") { [st.primary, st.secondary] = [st.secondary, st.primary]; renderToolBox(); if (st.tab === "palette") renderTab(); return; }
    if (k === "[" || k === "]") { st.brush = clampN(st.brush + (k === "]" ? 1 : -1), 1, 25); renderToolBox(); draw(); return; }
    if (k === "g") { st.show.grid = !st.show.grid; document.querySelector('[data-show="grid"]').checked = st.show.grid; draw(); return; }
    if (k === "0") { fit(); draw(); return; }
    if (k === "+" || k === "=") { st.view.scale = Math.min(10, st.view.scale * 1.25); draw(); return; }
    if (k === "-") { st.view.scale = Math.max(0.05, st.view.scale / 1.25); draw(); return; }
    const t = TOOLS.find((t) => t[3] === k);
    if (t) setTool(t[0]);
  }

  // ── Panels ──────────────────────────────────────────────────────────────────
  function setCursor() { $("meCanvas").style.cursor = st.tool === "hand" || st.space ? "grab" : st.tool === "select" ? "default" : "crosshair"; }
  function setTool(t) {
    st.tool = t; st.pasting = false; if (t !== "portal") st.pendingPortal = null;
    for (const b of $("meTools").querySelectorAll("button")) b.classList.toggle("on", b.dataset.tool === t);
    setCursor();
    renderToolBox(); draw();
  }
  function renderAll() { renderLayerSelect(); setTool(st.tool); renderSel(); renderTab(); renderStatus(); }
  function renderLayerSelect() {
    if (!st.doc) return;
    $("meLayer").innerHTML = st.doc.map.layers.map((l, i) => `<option value="${i}"${i === st.layer ? " selected" : ""}>${esc(l.name)} · ${l.width}×${l.height}${l.underground ? " · sotterranea" : ""}</option>`).join("");
  }
  const tileImgHtml = (ch, cls = "me-t s") => `<img class="${cls}" src="${artUrl(st.leg[ch])}" alt="">`;
  const tileSelect = (id, cur) => `<select id="${id}">${st.doc.map.legend.map((t) => `<option value="${esc(t.ch)}"${t.ch === cur ? " selected" : ""}>${esc(t.name || t.id)} (${esc(t.ch)})</option>`).join("")}</select>`;
  function renderToolBox() {
    if (!st.doc) return;
    const o = st.opt, box = $("meToolBox"), t = TOOLS.find((x) => x[0] === st.tool);
    const swatch = `<div class="me-swatch"><img src="${artUrl(st.leg[st.primary])}" title="primaria (tasto sinistro)"><div><b>${esc(tileName(st.primary))}</b><div class="muted">sinistro</div></div>` +
      `<img class="s2" src="${artUrl(st.leg[st.secondary])}" title="secondaria (tasto destro)"><div><b>${esc(tileName(st.secondary))}</b><div class="muted">destro</div></div><button id="meSwap" title="X">⇄</button></div>`;
    let h = `<div class="row" style="justify-content:space-between"><b>${esc(t[2])}</b></div>`;
    const facs = Object.values(C("factions")).map((f) => [f.id, f.name]);
    if (["brush", "eraser", "line", "rect", "ellipse", "fill"].includes(st.tool)) {
      h += swatch;
      if (["brush", "eraser", "line"].includes(st.tool)) h += `<div class="row" style="margin-top:6px">Grandezza <input id="meBrush" type="range" min="1" max="25" value="${st.brush}"> <b>${st.brush}</b> <label><input type="checkbox" id="meRound"${st.round ? " checked" : ""}> tonda</label></div>`;
      if (["rect", "ellipse"].includes(st.tool)) h += `<label><input type="checkbox" id="meOutline"${st.outline ? " checked" : ""}> solo il bordo</label> <span class="muted">· Maiusc: quadrato/cerchio</span>`;
      if (st.tool === "line") h += `<div class="muted">Maiusc: dritta o a 45°. Con la grandezza fai strade e fiumi.</div>`;
      if (st.tool === "fill") h += `<label><input type="checkbox" id="meFillAll"${st.fillAll ? " checked" : ""}> sostituisci ovunque in questa mappa</label>`;
    } else if (st.tool === "pick") h += swatch + `<div class="muted">Sinistro: prende la primaria; destro: la secondaria.</div>`;
    else if (st.tool === "room") {
      h += `<div class="me-form">Muri ${tileSelect("meWall", o.wall)} Pavimento ${tileSelect("meFloor", o.floor)} Porta ${tileSelect("meDoor", o.door)}</div><div class="muted">Trascina per disegnare la stanza: la porta va al centro del lato in basso.</div>`;
    } else if (st.tool === "area") {
      const it = areaItems(), sum = areaSummary(it);
      h += st.area ? `<div>Area <b>${st.area.w}×${st.area.h}</b> in (${st.area.x}, ${st.area.y})${sum ? ` · dentro: <b>${esc(sum)}</b>` : " · nessun elemento dentro"}</div>` : `<div class="muted">Trascina per selezionare un'area: prende le caselle e gli elementi che ci stanno dentro (edifici, NPC, oggetti nascosti, passaggi; le zone se ci stanno per intero).</div>`;
      h += `<div class="row" style="margin-top:6px">Prende: ${AREA_KINDS.map(([k, t]) => `<label><input type="checkbox" data-ak="${k}"${st.areaKinds[k] ? " checked" : ""}> ${t}</label>`).join(" ")}</div>`;
      if (st.area) {
        h += `<div class="row"><button class="danger" data-a="delitems"${it.n ? "" : " disabled"}>Elimina gli elementi${it.n ? ` (${it.n})` : ""}</button><button data-a="clear"${st.areaKinds.tiles ? "" : " disabled"}>Svuota le caselle</button><button class="danger" data-a="delall">Elimina tutto</button></div>
          <div class="row"><button data-a="copy">Copia</button><button data-a="cut">Taglia</button><button data-a="paste"${st.clip ? "" : " disabled"}>Incolla</button><button data-a="fillp">Riempi con la primaria</button><button data-a="zone">Crea una zona qui</button></div>
          <div class="muted">Canc elimina gli elementi (o svuota le caselle, se non ce ne sono) · le frecce spostano gli elementi · Esc toglie la selezione.</div>`;
      } else if (st.clip) h += `<div class="row"><button data-a="paste">Incolla</button></div>`;
      if (st.clip) h += `<div class="row">Appunti ${st.clip.w}×${st.clip.h}${st.clip.items?.length ? `, ${st.clip.items.length} elementi` : ""}: <button data-a="h">Specchia ↔</button><button data-a="v">Specchia ↕</button><button data-a="r">Ruota ↻</button></div>`;
    } else if (st.tool === "zone") h += `<div class="muted">Trascina un rettangolo: diventa una zona (quartiere, stanza, area di apparizione…). Poi modifica nome, id e tag qui sotto.</div>`;
    else if (st.tool === "portal") h += st.pendingPortal ? `<div>Primo capo: ${esc(st.doc.map.layers[st.pendingPortal.layer].name)} (${st.pendingPortal.x}, ${st.pendingPortal.y}). <b>Clicca l'altro capo</b> (cambia mappa in alto se serve). Esc annulla.</div>` : `<div class="muted">Clicca un capo e poi l'altro: scale, botole, porte fra le mappe. Una pedina sul capo passa dall'altra parte.</div>`;
    else if (st.tool === "building") {
      h += `<div class="me-form">Edificio <select id="meBld">${opts(Object.values(C("buildings")).sort((a, b) => a.name.localeCompare(b.name)).map((b) => [b.id, b.name]), o.building)}</select>
        Proprietario <select id="meOwner">${opts(facs, o.owner, "nessuno")}</select> Nome <input id="meBName" value="${esc(o.bname)}" placeholder="(predefinito)"></div><div class="muted">Clicca la casella della porta (il punto in basso al centro).</div>`;
    } else if (st.tool === "npc") {
      h += `<div class="me-form">Template <select id="meTpl">${opts(Object.values(C("templates")).sort((a, b) => (a.name || a.id).localeCompare(b.name || b.id)).map((t) => [t.id, (t.name || t.id) + (t.unique ? " ★" : "")]), o.template)}</select>
        Nome <input id="meNpcName" value="${esc(o.npcName)}" placeholder="(dal template)"> Fazione <select id="meFac">${opts(facs, o.faction, "quella del template")}</select>
        Quante <input id="meCount" type="number" min="1" value="${o.count}"></div><div class="muted">Clicca dove nasce all'inizio della partita.</div>`;
    } else if (st.tool === "hidden") {
      h += `<div class="me-form">Oggetto <select id="meHideItem">${opts(Object.values(C("items")).sort((a, b) => a.name.localeCompare(b.name)).map((i) => [i.id, i.name]), o.hideItem)}</select> Quantità <input id="meHideQty" type="number" min="1" value="${o.hideQty}"></div>
        <div class="muted">Clicca dove nasconderlo. I giocatori non lo vedono; una pedina che ci passa accanto può trovarlo (più facilmente con tanta percezione) e se lo prende. Altri oggetti si aggiungono dalla scheda del nascondiglio.</div>`;
    } else if (st.tool === "hand") h += `<div class="muted">Trascina la mappa per spostarti. Funziona anche con qualsiasi altro strumento tenendo premuto <b>spazio</b> o col tasto centrale del mouse; la rotella fa lo zoom, la miniatura in basso salta a un punto.</div>`;
    else h += `<div class="muted">Clicca per selezionare edifici, NPC, oggetti nascosti, passaggi e zone; trascina per spostarli (le zone selezionate si ridimensionano dagli angoli). Canc elimina, frecce spostano di una casella.</div>`;
    box.innerHTML = h;
    const bind = (id, f, ev = "onchange") => { const el = $(id); if (el) el[ev] = () => f(el); };
    bind("meSwap", () => { [st.primary, st.secondary] = [st.secondary, st.primary]; renderToolBox(); if (st.tab === "palette") renderTab(); }, "onclick");
    bind("meBrush", (el) => { st.brush = Number(el.value); renderToolBox(); }, "oninput");
    bind("meRound", (el) => { st.round = el.checked; });
    bind("meOutline", (el) => { st.outline = el.checked; });
    bind("meFillAll", (el) => { st.fillAll = el.checked; });
    bind("meWall", (el) => (o.wall = el.value)); bind("meFloor", (el) => (o.floor = el.value)); bind("meDoor", (el) => (o.door = el.value));
    bind("meBld", (el) => (o.building = el.value)); bind("meOwner", (el) => (o.owner = el.value)); bind("meBName", (el) => (o.bname = el.value), "oninput");
    bind("meTpl", (el) => (o.template = el.value)); bind("meNpcName", (el) => (o.npcName = el.value), "oninput"); bind("meFac", (el) => (o.faction = el.value)); bind("meCount", (el) => (o.count = el.value), "oninput");
    bind("meHideItem", (el) => (o.hideItem = el.value)); bind("meHideQty", (el) => (o.hideQty = el.value), "oninput");
    for (const c of box.querySelectorAll("[data-ak]")) c.onchange = () => { st.areaKinds[c.dataset.ak] = c.checked; renderToolBox(); draw(); };
    for (const b of box.querySelectorAll("[data-a]")) b.onclick = () => {
      const a = b.dataset.a;
      if (a === "copy") copyArea(false); else if (a === "cut") copyArea(true);
      else if (a === "delitems") deleteAreaItems();
      else if (a === "delall" && st.area) {
        const it = areaItems(true);
        if (it.n > 5 && !confirm(`Eliminare ${areaSummary(it)} e svuotare le caselle?`)) return;
        beginTiles(); forArea((x, y) => setTile(x, y, st.secondary)); endTiles("svuota l'area"); drawMini();
        if (it.n) change(`elimina ${it.n} elementi`, () => { removeItems(it); });
        st.sel = null; renderSel(); renderToolBox(); if (st.tab !== "palette") renderTab();
      }
      else if (a === "paste") { st.pasting = true; toast("Clic per incollare, R ruota, Esc per finire"); }
      else if (a === "fillp" && st.area) { beginTiles(); forArea((x, y) => setTile(x, y, st.primary)); endTiles("riempi l'area"); drawMini(); }
      else if (a === "clear" && st.area) { beginTiles(); forArea((x, y) => setTile(x, y, st.secondary)); endTiles("svuota l'area"); drawMini(); }
      else if (a === "zone" && st.area) newZone({ x: st.area.x, y: st.area.y }, { x: st.area.x + st.area.w - 1, y: st.area.y + st.area.h - 1 });
      else if ("hvr".includes(a)) { transformClip(a); renderToolBox(); }
      draw();
    };
  }

  // Selected element's card.
  function renderSel() {
    const box = $("meSelBox"), s = st.sel;
    if (!s || !st.doc) { box.hidden = true; return; }
    box.hidden = false;
    const facs = Object.values(C("factions")).map((f) => [f.id, f.name]);
    const zoneOpts = (cur, li) => opts([[st.doc.map.layers[li].id, "(tutta la mappa) " + st.doc.map.layers[li].name], ...zones().filter((z) => z.layer === st.doc.map.layers[li].id).map((z) => [z.id, z.name + " · " + z.id])], cur);
    let h = "";
    if (s.type === "other") {
      const P = pl(s.o.placement);
      h = `<b>${esc(P.b.name || P.b.template || P.b.building)}</b> <span class="muted">da ${esc(s.o.file)} (sola lettura)</span><pre class="muted" style="margin-top:6px">${esc(JSON.stringify(s.o.placement, null, 1))}</pre>`;
      box.innerHTML = h; return;
    }
    if (s.type === "pl") {
      const p = st.doc.placements[s.i]; if (!p) { st.sel = null; box.hidden = true; return; }
      const P = pl(p), b = P.b, li = P.pos?.layer ?? st.layer;
      if (P.kind === "Building" || P.kind === "Hidden") {
        const stock = Object.entries(b.stock ?? {});
        h += `<div class="row" style="justify-content:space-between"><b>${P.kind === "Hidden" ? "Oggetto nascosto" : "Edificio"}</b><span><button id="meDup">Duplica</button> <button class="danger" id="meDel">Elimina</button></span></div><div class="me-form">`;
        if (P.kind === "Building") h += `Tipo <select data-f="building">${opts(Object.values(C("buildings")).map((x) => [x.id, x.name]), b.building)}</select>
          Nome <input data-f="name" value="${esc(b.name ?? "")}" placeholder="${esc(C("buildings")[b.building]?.name ?? "")}">
          Proprietario <select data-f="owner_faction">${opts(facs, b.owner_faction ?? "", "nessuno")}</select>
          Di un personaggio <select data-f="owner_template">${opts(Object.values(C("templates")).filter((t) => t.unique).map((t) => [t.id, t.name || t.id]), b.owner_template ?? "", "nessuno")}</select>
          Nascosto <label><input type="checkbox" data-f="hidden"${b.hidden ? " checked" : ""}> finché qualcuno non lo trova</label>`;
        h += `Zona <select data-f="zone">${zoneOpts(b.zone, li)}</select> Posizione <span>(${b.at?.[0] ?? "?"}, ${b.at?.[1] ?? "?"})</span></div>`;
        h += `<h4>${P.kind === "Hidden" ? "Cosa c'è nascosto" : "Contenuto"}</h4>` + (P.kind === "Building" && !b.stock ? `<div class="muted">Quello predefinito dell'edificio. <button id="meStockOwn">Personalizza</button></div>` :
          `<table class="ed">${stock.map(([k, q]) => `<tr><td>${esc(C("items")[k]?.name ?? k)}</td><td><input class="num" type="number" min="0" data-q="${esc(k)}" value="${q}"></td><td><button class="x" data-rm="${esc(k)}">✕</button></td></tr>`).join("")}</table>
          <div class="row"><select id="meAddItem">${opts(Object.values(C("items")).sort((a, b) => a.name.localeCompare(b.name)).map((i) => [i.id, i.name]), "")}</select><button id="meAddItemBtn">Aggiungi</button>${P.kind === "Building" ? ` <button id="meStockDef">Usa quello predefinito</button>` : ""}</div>`);
      } else if (P.kind === "Entity") {
        const players = st.doc.placements.concat(st.others.map((o) => o.placement)).filter((x) => x.Player).map((x) => [x.Player.id, x.Player.name]);
        h += `<div class="row" style="justify-content:space-between"><b>NPC</b><span><button id="meDup">Duplica</button> <button class="danger" id="meDel">Elimina</button></span></div><div class="me-form">
          Template <select data-f="template">${opts(Object.values(C("templates")).map((t) => [t.id, t.name || t.id]), b.template)}</select>
          Nome <input data-f="name" value="${esc(b.name ?? "")}" placeholder="${esc(C("templates")[b.template]?.name ?? "")}">
          Fazione <select data-f="faction">${opts(facs, b.faction ?? "", "quella del template")}</select>
          Quanti <input data-f="count" type="number" min="1" value="${b.count ?? 1}">
          Zona <select data-f="zone">${zoneOpts(b.zone, li)}</select>
          Posizione <label><input type="checkbox" id="meRandom"${b.at ? "" : " checked"}> a caso nella zona</label>
          Legato alla zona <select id="meTether">${opts(zones().map((z) => [z.id, z.name]), b.tether?.zone ?? "", "no (gira libero)")}</select>
          Capo del giocatore <select data-f="leader_of">${opts(players, b.leader_of ?? "", "no")}</select></div>
          ${b.tether ? `<div class="muted">Liberato quando: <code>${esc(JSON.stringify(b.tether.release))}</code></div>` : ""}`;
      }
    } else if (s.type === "zone") {
      const z = zones().find((z) => z.id === s.id); if (!z) { st.sel = null; box.hidden = true; return; }
      const users = st.doc.placements.filter((p) => pl(p).b.zone === z.id).length;
      h += `<div class="row" style="justify-content:space-between"><b>Zona</b><span><button id="meZoneArea">Seleziona l'area</button> <button class="danger" id="meDel">Elimina</button></span></div><div class="me-form">
        Nome <input data-z="name" value="${esc(z.name)}"> Id <input data-z="id" value="${esc(z.id)}"> Tag <input data-z="tags" value="${esc((z.tags ?? []).join(", "))}" placeholder="pubblico, quartiere…">
        Rettangolo <span><input class="num" data-r="0" value="${z.rect[0]}" style="width:52px"> <input class="num" data-r="1" value="${z.rect[1]}" style="width:52px"> <input class="num" data-r="2" value="${z.rect[2]}" style="width:52px"> <input class="num" data-r="3" value="${z.rect[3]}" style="width:52px"></span></div>
        <div class="muted">x, y, larghezza, altezza · ${users} piazzamenti in questa zona. Il tag «quartiere» la rende conquistabile.</div>`;
    } else if (s.type === "portal") {
      const p = st.doc.map.portals[s.i]; if (!p) { st.sel = null; box.hidden = true; return; }
      const end = (k) => `<select data-pe="${k}">${opts(st.doc.map.layers.map((l) => [l.id, l.name]), p[k][0])}</select> <input class="num" data-px="${k}" value="${p[k][1]}" style="width:52px"> <input class="num" data-py="${k}" value="${p[k][2]}" style="width:52px"> <button data-go="${k}">Vai</button>`;
      h += `<div class="row" style="justify-content:space-between"><b>Passaggio</b><button class="danger" id="meDel">Elimina</button></div><div class="me-form">Nome <input data-pn value="${esc(p.name)}"> Capo A <span>${end("a")}</span> Capo B <span>${end("b")}</span></div>`;
    }
    box.innerHTML = h;
    wireSel(box);
  }
  function wireSel(box) {
    const s = st.sel, redraw = () => { renderSel(); draw(); if (st.tab !== "palette") renderTab(); };
    const del = box.querySelector("#meDel"); if (del) del.onclick = deleteSel;
    if (s.type === "pl") {
      const p = st.doc.placements[s.i], b = pl(p).b;
      const dup = box.querySelector("#meDup");
      if (dup) dup.onclick = () => change("duplica", () => {
        const c = JSON.parse(JSON.stringify(p)), cb = Object.values(c)[0];
        if (cb.at) cb.at = [cb.at[0] + 1, cb.at[1]];
        st.doc.placements.push(c); st.sel = { type: "pl", i: st.doc.placements.length - 1 };
      }) !== false && redraw();
      for (const el of box.querySelectorAll("[data-f]")) el.onchange = () => change("modifica", () => {
        const f = el.dataset.f;
        let v = el.type === "checkbox" ? el.checked : el.value;
        if (f === "count") v = Math.max(1, Number(v) || 1);
        if (v === "" || v === false) delete b[f]; else b[f] = v;
      }) !== false && redraw();
      const rnd = box.querySelector("#meRandom");
      if (rnd) rnd.onchange = () => change("posizione", () => { if (rnd.checked) delete b.at; else { const z = zoneInfo(b.zone); b.at = [z.x + Math.floor(z.w / 2), z.y + Math.floor(z.h / 2)]; } }) !== false && redraw();
      const teth = box.querySelector("#meTether");
      if (teth) teth.onchange = () => change("legame", () => { if (teth.value) b.tether = { zone: teth.value, release: b.tether?.release ?? "Never" }; else delete b.tether; }) !== false && redraw();
      for (const el of box.querySelectorAll("[data-q]")) el.onchange = () => change("quantità", () => { b.stock[el.dataset.q] = Math.max(0, Number(el.value) || 0); }) !== false && redraw();
      for (const el of box.querySelectorAll("[data-rm]")) el.onclick = () => change("togli oggetto", () => { delete b.stock[el.dataset.rm]; }) !== false && redraw();
      const add = box.querySelector("#meAddItemBtn");
      if (add) add.onclick = () => { const it = box.querySelector("#meAddItem").value; if (it) change("aggiungi oggetto", () => { b.stock ??= {}; b.stock[it] = (b.stock[it] ?? 0) + 1; }) !== false && redraw(); };
      const own = box.querySelector("#meStockOwn");
      if (own) own.onclick = () => change("contenuto", () => { b.stock = { ...(C("buildings")[b.building]?.stock ?? {}) }; }) !== false && redraw();
      const def = box.querySelector("#meStockDef");
      if (def) def.onclick = () => change("contenuto", () => { delete b.stock; }) !== false && redraw();
    } else if (s.type === "zone") {
      const z = zones().find((z) => z.id === s.id);
      box.querySelector("#meZoneArea").onclick = () => { st.area = { x: z.rect[0], y: z.rect[1], w: z.rect[2], h: z.rect[3] }; setTool("area"); };
      for (const el of box.querySelectorAll("[data-z]")) el.onchange = () => {
        const f = el.dataset.z, v = el.value.trim();
        if (f === "id") {
          if (!/^[a-z0-9_]+$/.test(v)) { toast("L'id può avere solo lettere minuscole, cifre e _", true); redraw(); return; }
          if (v !== z.id && (zones().some((x) => x.id === v) || layerIdx(v) >= 0)) { toast("Esiste già una zona o mappa con questo id", true); redraw(); return; }
          const old = z.id;
          change("rinomina la zona", () => {
            z.id = v;
            for (const p of st.doc.placements) { const b = pl(p).b; if (b.zone === old) b.zone = v; if (b.tether?.zone === old) b.tether.zone = v; }
            for (const n of st.doc.map.networks) n.zones = n.zones.map((x) => (x === old ? v : x));
          });
          st.sel = { type: "zone", id: v };
          if (st.others.some((o) => pl(o.placement).b.zone === old)) toast("Altri file usano ancora l'id vecchio: la verifica lo segnalerà", true);
        } else change("modifica la zona", () => { if (f === "tags") z.tags = v.split(",").map((t) => t.trim()).filter(Boolean); else z[f] = v; });
        redraw();
      };
      for (const el of box.querySelectorAll("[data-r]")) el.onchange = () => change("rettangolo", () => { z.rect[Number(el.dataset.r)] = Math.max(Number(el.dataset.r) >= 2 ? 1 : -9999, Number(el.value) || 0); }) !== false && redraw();
    } else if (s.type === "portal") {
      const p = st.doc.map.portals[s.i];
      box.querySelector("[data-pn]").onchange = (e) => change("nome del passaggio", () => { p.name = e.target.value; }) !== false && redraw();
      for (const el of box.querySelectorAll("[data-pe]")) el.onchange = () => change("passaggio", () => { p[el.dataset.pe][0] = el.value; }) !== false && redraw();
      for (const el of box.querySelectorAll("[data-px],[data-py]")) el.onchange = () => change("passaggio", () => { const k = el.dataset.px ?? el.dataset.py; p[k][el.dataset.px ? 1 : 2] = Number(el.value) || 0; }) !== false && redraw();
      for (const el of box.querySelectorAll("[data-go]")) el.onclick = () => { const e = p[el.dataset.go]; const li = layerIdx(e[0]); if (li !== st.layer) setLayer(li); st.sel = { type: "portal", i: s.i, end: el.dataset.go }; centerOn(e[1], e[2]); renderSel(); };
    }
  }

  // Side tabs.
  function renderTab() {
    if (!st.doc) return;
    for (const b of $("meTabs").querySelectorAll("button")) b.classList.toggle("on", b.dataset.tab === st.tab);
    ({ palette: tabPalette, items: tabItems, zones: tabZones, map: tabMap, legend: tabLegend, check: () => showCheck(st.lastCheck ?? [], false) })[st.tab]();
  }
  function groupOf(t) {
    if (t.group) return t.group;
    return /piastrell|parquet|marmo|tappeto|moquette|cemento|asfalto|lastric|piazza/.test(t.id) ? "Pavimenti" : "Altro";
  }
  function tabPalette() {
    const q = (st.palQ ?? "").toLowerCase();
    const groups = {};
    for (const t of st.doc.map.legend) if (!q || (t.name || "").toLowerCase().includes(q) || t.id.includes(q) || t.ch === q) (groups[groupOf(t)] ??= []).push(t);
    const order = [...GROUP_ORDER, ...Object.keys(groups).filter((g) => !GROUP_ORDER.includes(g))];
    let h = `<div class="row"><input id="mePalQ" placeholder="cerca casella…" value="${esc(st.palQ ?? "")}" style="flex:1"></div>`;
    for (const g of order) {
      if (!groups[g]) continue;
      h += `<h4>${esc(g)}</h4><div class="me-pal">` + groups[g].map((t) => `<button data-ch="${esc(t.ch)}" class="${t.ch === st.primary ? "p" : ""} ${t.ch === st.secondary ? "s2" : ""}" title="${esc(t.name || t.id)} · '${esc(t.ch)}'${t.walkable === false ? " · solido" : ""}${t.dig_to ? " · si scava" : ""}${t.color ? " · " + t.color : ""}"><img src="${artUrl(t)}" alt="">${t.walkable === false ? '<span class="solid">■</span>' : ""}<span>${esc((t.name || t.id).slice(0, 9))}</span></button>`).join("") + `</div>`;
    }
    const base = st.leg[st.primary];
    h += `<h4>Nuova variante colorata</h4><div class="me-form">Da <select id="meVarBase">${st.doc.map.legend.filter((t) => !t.color).map((t) => `<option value="${esc(t.ch)}"${t.id === (base?.sprite || base?.id) || (t.ch === st.primary) ? " selected" : ""}>${esc(t.name || t.id)}</option>`).join("")}</select>
      Colore <span><input type="color" id="meVarColor" value="${st.varColor ?? "#7ab8e8"}"> <img class="me-t s" id="meVarPrev" alt=""></span> Nome <input id="meVarName" placeholder="es. Piastrelle azzurre"></div>
      <div class="row" style="margin-top:4px"><button id="meVarAdd">Crea e usa</button> <span class="muted">Si colora la figura della casella scelta (meglio se chiara: piastrelle, marmo, parete).</span></div>`;
    $("meTab").innerHTML = h;
    $("mePalQ").oninput = (e) => { st.palQ = e.target.value; tabPalette(); const i = $("mePalQ"); i.focus(); i.setSelectionRange(i.value.length, i.value.length); };
    for (const b of $("meTab").querySelectorAll(".me-pal button")) {
      b.onclick = () => { st.primary = b.dataset.ch; if (["hand", "select", "pick", "area", "zone", "portal", "building", "npc", "hidden", "room"].includes(st.tool)) setTool("brush"); renderToolBox(); tabPalette(); };
      b.oncontextmenu = (e) => { e.preventDefault(); st.secondary = b.dataset.ch; renderToolBox(); tabPalette(); };
    }
    const prev = () => { const t = st.leg[$("meVarBase").value]; $("meVarPrev").src = art({ ...t, sprite: t.sprite || t.id, color: $("meVarColor").value }).toDataURL(); };
    $("meVarBase").onchange = prev; $("meVarColor").oninput = () => { st.varColor = $("meVarColor").value; prev(); }; prev();
    $("meVarAdd").onclick = () => {
      const t = st.leg[$("meVarBase").value], color = $("meVarColor").value;
      const name = $("meVarName").value.trim() || `${t.name || t.id} ${color}`;
      const ch = freeChar(); if (!ch) { toast("Caratteri finiti", true); return; }
      let id = slug(name), n = 2; while (st.doc.map.legend.some((x) => x.id === id)) id = slug(name) + "_" + n++;
      change("nuova casella colorata", () => { st.doc.map.legend.push({ ch, id, name, walkable: t.walkable !== false, sprite: t.sprite || t.id, color, group: groupOf(t) }); rebuildLegend(); });
      st.primary = ch; renderToolBox(); tabPalette(); toast(`Creata «${name}»`);
    };
  }
  /** A character no legend entry uses (printable, one UTF-16 unit, no spaces). */
  function freeChar() {
    const used = new Set(st.doc.map.legend.map((t) => t.ch));
    const pool = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!$%&*+-/;?@[]^`{}()'";
    for (const c of pool) if (!used.has(c)) return c;
    for (let i = 0xc0; i < 0x2ff; i++) { const c = String.fromCharCode(i); if (!used.has(c) && i !== 0xd7 && i !== 0xf7) return c; }
    for (let i = 0x391; i < 0x52f; i++) { const c = String.fromCharCode(i); if (!used.has(c) && /\p{L}/u.test(c)) return c; }
    return null;
  }
  function tabItems() {
    const q = (st.itemQ ?? "").toLowerCase(), kinds = st.itemKinds ??= { Building: true, Entity: true, Hidden: true, others: true };
    const label = (P) => P.kind === "Entity" ? (P.b.name || C("templates")[P.b.template]?.name || P.b.template) + ((P.b.count ?? 1) > 1 ? ` ×${P.b.count}` : "")
      : P.kind === "Hidden" ? "✦ " + Object.entries(P.b.stock ?? {}).map(([k, n]) => `${n}× ${C("items")[k]?.name ?? k}`).join(", ")
      : (P.b.hidden ? "🔒 " : "") + (P.b.name || C("buildings")[P.b.building]?.name || P.b.building);
    const icon = { Building: "🏠", Entity: "☺", Hidden: "✦", Player: "♟" };
    const rows = [];
    st.doc.placements.forEach((p, i) => { const P = pl(p); if (!P.pos || P.pos.layer !== st.layer || !kinds[P.kind]) return; const t = label(P); if (!q || t.toLowerCase().includes(q)) rows.push({ i, P, t }); });
    const oth = kinds.others ? st.others.map((o) => ({ o, P: pl(o.placement) })).filter((x) => x.P.pos?.layer === st.layer && x.P.kind !== "Player").filter((x) => !q || label(x.P).toLowerCase().includes(q)) : [];
    const counts = { Building: 0, Entity: 0, Hidden: 0 };
    for (const p of st.doc.placements) { const P = pl(p); if (P.pos?.layer === st.layer && P.kind in counts) counts[P.kind]++; }
    $("meTab").innerHTML = `<div class="row"><input id="meItemQ" placeholder="cerca…" value="${esc(st.itemQ ?? "")}" style="flex:1"></div>
      <div class="row">${[["Building", "edifici"], ["Entity", "NPC"], ["Hidden", "nascosti"], ["others", "altri file"]].map(([k, t]) => `<label><input type="checkbox" data-k="${k}"${kinds[k] ? " checked" : ""}> ${t}${counts[k] != null ? ` (${counts[k]})` : ""}</label>`).join(" ")}</div>
      <div class="me-list">${rows.map((r) => `<div class="it${st.sel?.type === "pl" && st.sel.i === r.i ? " on" : ""}" data-i="${r.i}">${icon[r.P.kind]} <span style="flex:1">${esc(r.t)}</span><span class="muted">${r.P.random ? "a caso" : `${r.P.pos.x},${r.P.pos.y}`}</span></div>`).join("") || '<div class="muted" style="padding:6px">Niente in questa mappa.</div>'}
      ${oth.map((x, j) => `<div class="it" data-o="${st.others.indexOf(x.o)}" style="opacity:.7">🔒 ${icon[x.P.kind]} <span style="flex:1">${esc(label(x.P))}</span><span class="muted">${esc(x.o.file)}</span></div>`).join("")}</div>
      <div class="muted" style="margin-top:6px">Quelli con 🔒 stanno in altri file dei contenuti: si vedono ma non si modificano da qui.</div>`;
    $("meItemQ").oninput = (e) => { st.itemQ = e.target.value; tabItems(); const i = $("meItemQ"); i.focus(); i.setSelectionRange(i.value.length, i.value.length); };
    for (const c of $("meTab").querySelectorAll("[data-k]")) c.onchange = () => { kinds[c.dataset.k] = c.checked; tabItems(); };
    for (const d of $("meTab").querySelectorAll("[data-i]")) d.onclick = () => { st.sel = { type: "pl", i: Number(d.dataset.i) }; const P = pl(st.doc.placements[st.sel.i]); centerOn(P.pos.x, P.pos.y); renderSel(); tabItems(); };
    for (const d of $("meTab").querySelectorAll("[data-o]")) d.onclick = () => { const o = st.others[Number(d.dataset.o)]; st.sel = { type: "other", o }; const P = pl(o.placement); centerOn(P.pos.x, P.pos.y); renderSel(); };
  }
  function tabZones() {
    const l = L(), list = zones().filter((z) => z.layer === l.id).sort((a, b) => b.rect[2] * b.rect[3] - a.rect[2] * a.rect[3]);
    let h = `<div class="muted">Trascina con lo strumento Zona (Z) per crearne una.</div><div class="me-list" style="margin-top:6px">` +
      list.map((z) => `<div class="it${st.sel?.type === "zone" && st.sel.id === z.id ? " on" : ""}" data-z="${esc(z.id)}"><span style="flex:1">${esc(z.name)}</span><span class="muted">${esc(z.id)} · ${z.rect[2]}×${z.rect[3]}</span></div>`).join("") + `</div>`;
    h += `<h4>Reti (acquedotto, fogne…)</h4>` + st.doc.map.networks.map((n, i) => `<div class="card"><div class="row"><b>${esc(n.name)}</b> <span class="muted">${esc(n.id)} · ${esc(n.vector ?? "")}</span> <button class="x" data-nrm="${i}">✕</button></div>
      <div>${n.zones.map((z) => `<span class="chip">${esc(zones().find((x) => x.id === z)?.name ?? z)} <button class="x" data-nz="${i}" data-zid="${esc(z)}">✕</button></span>`).join("")}</div>
      <div class="row" style="margin-top:4px"><select data-nadd="${i}">${opts(zones().filter((z) => !n.zones.includes(z.id)).map((z) => [z.id, z.name]), "", "aggiungi una zona…")}</select></div></div>`).join("") +
      `<div class="row"><input id="meNetName" placeholder="nome della rete"><button id="meNetAdd">Nuova rete</button></div>`;
    $("meTab").innerHTML = h;
    for (const d of $("meTab").querySelectorAll("[data-z]")) d.onclick = () => { st.sel = { type: "zone", id: d.dataset.z }; const z = zones().find((z) => z.id === d.dataset.z); centerOn(z.rect[0] + z.rect[2] / 2, z.rect[1] + z.rect[3] / 2, false); renderSel(); tabZones(); };
    for (const b of $("meTab").querySelectorAll("[data-nrm]")) b.onclick = () => { if (confirm("Eliminare la rete?")) { change("elimina la rete", () => { st.doc.map.networks.splice(Number(b.dataset.nrm), 1); }); tabZones(); } };
    for (const b of $("meTab").querySelectorAll("[data-nz]")) b.onclick = () => { change("rete", () => { const n = st.doc.map.networks[Number(b.dataset.nz)]; n.zones = n.zones.filter((z) => z !== b.dataset.zid); }); tabZones(); };
    for (const s of $("meTab").querySelectorAll("[data-nadd]")) s.onchange = () => { if (s.value) { change("rete", () => { st.doc.map.networks[Number(s.dataset.nadd)].zones.push(s.value); }); tabZones(); } };
    $("meNetAdd").onclick = () => { const name = $("meNetName").value.trim(); if (!name) return; change("nuova rete", () => { st.doc.map.networks.push({ id: slug(name), name, zones: [] }); }); tabZones(); };
  }
  function tabMap() {
    const l = L(), m = st.doc.map;
    const edges = m.edges.map((e, i) => ({ e, i })).filter(({ e }) => e.a === l.id || e.b === l.id);
    const portals = m.portals.map((p, i) => ({ p, i })).filter(({ p }) => p.a[0] === l.id || p.b[0] === l.id);
    const anchor = st.anchor ?? 4;
    $("meTab").innerHTML = `<h4>Questa mappa</h4><div class="me-form">
      Id <input id="mlId" value="${esc(l.id)}"> Nome <input id="mlName" value="${esc(l.name)}"> Tag <input id="mlTags" value="${esc((l.tags ?? []).join(", "))}">
      Tipo <span><label><input type="checkbox" id="mlUnder"${l.underground ? " checked" : ""}> sotterranea</label> <label><input type="checkbox" id="mlIndoor"${l.indoor ? " checked" : ""}> al chiuso</label></span>
      Profondità <input id="mlDepth" type="number" value="${l.depth ?? 0}"></div>
      <h4>Dimensioni</h4><div class="row">Larghezza <input id="mlW" type="number" min="4" value="${l.width}" style="width:70px"> Altezza <input id="mlH" type="number" min="4" value="${l.height}" style="width:70px">
      <span class="me-anchor">${[0, 1, 2, 3, 4, 5, 6, 7, 8].map((k) => `<button data-an="${k}" class="${k === anchor ? "on" : ""}" title="dove resta il contenuto">${k === anchor ? "●" : ""}</button>`).join("")}</span></div>
      <div class="row">Le caselle nuove sono quelle secondarie (${tileImgHtml(st.secondary)} ${esc(tileName(st.secondary))}). <button id="mlResize">Ridimensiona</button></div>
      <h4>Bordi con altre mappe</h4>
      ${edges.map(({ e, i }) => `<div class="row">${e.a === l.id ? `lato <b>${SIDE_IT[e.side]}</b> → ${esc(m.layers[layerIdx(e.b)]?.name ?? e.b)}` : `lato <b>${SIDE_IT[{ North: "South", South: "North", East: "West", West: "East" }[e.side]]}</b> → ${esc(m.layers[layerIdx(e.a)]?.name ?? e.a)}`} · spostamento <input class="num" data-eo="${i}" value="${e.offset ?? 0}" style="width:56px"> <button class="x" data-erm="${i}">✕</button></div>`).join("") || '<div class="muted">Nessuno.</div>'}
      <div class="row">Lato <select id="meEdgeSide">${SIDES.map((s) => `<option value="${s}">${SIDE_IT[s]}</option>`).join("")}</select> tocca <select id="meEdgeTo">${opts(m.layers.filter((x) => x.id !== l.id).map((x) => [x.id, x.name]), "")}</select><button id="meEdgeAdd">Aggiungi</button></div>
      <div class="muted">Le caselle calpestabili affacciate sul bordo diventano passaggi: si esce da un lato e si entra dall'altra mappa.</div>
      <h4>Passaggi</h4><div class="me-list">${portals.map(({ p, i }) => { const here = p.a[0] === l.id ? p.a : p.b, there = p.a[0] === l.id ? p.b : p.a; return `<div class="it" data-pi="${i}" data-x="${here[1]}" data-y="${here[2]}">⇅ <span style="flex:1">${esc(p.name)}</span><span class="muted">→ ${esc(m.layers[layerIdx(there[0])]?.name ?? there[0])}</span></div>`; }).join("") || '<div class="muted" style="padding:6px">Nessuno.</div>'}</div>
      <h4>Nuova mappa</h4><div class="me-form">Id <input id="mnId" placeholder="es. bunker_overmind"> Nome <input id="mnName" placeholder="es. Bunker dell'Overmind">
      Dimensioni <span><input id="mnW" type="number" value="64" style="width:70px"> × <input id="mnH" type="number" value="40" style="width:70px"></span>
      Riempita di <span>${tileImgHtml(st.secondary)} ${esc(tileName(st.secondary))} <span class="muted">(la secondaria)</span></span>
      Tipo <span><label><input type="checkbox" id="mnUnder" checked> sotterranea (dungeon)</label></span></div>
      <div class="row" style="margin-top:4px"><button id="mnAdd">Crea</button> <button id="mnDup">Duplica questa mappa</button> <button class="danger" id="mlDel">Elimina questa mappa</button></div>`;
    const set = (id, f) => { $(id).onchange = () => { change("proprietà della mappa", () => { f($(id)); }); renderLayerSelect(); tabMap(); }; };
    set("mlName", (el) => (l.name = el.value)); set("mlTags", (el) => (l.tags = el.value.split(",").map((t) => t.trim()).filter(Boolean)));
    set("mlUnder", (el) => (l.underground = el.checked)); set("mlIndoor", (el) => (l.indoor = el.checked)); set("mlDepth", (el) => (l.depth = Number(el.value) || 0));
    $("mlId").onchange = () => renameLayer(l, $("mlId").value.trim());
    for (const b of $("meTab").querySelectorAll("[data-an]")) b.onclick = () => { st.anchor = Number(b.dataset.an); tabMap(); };
    $("mlResize").onclick = () => resizeLayer(Number($("mlW").value), Number($("mlH").value), anchor);
    for (const i of $("meTab").querySelectorAll("[data-eo]")) i.onchange = () => { change("bordo", () => { m.edges[Number(i.dataset.eo)].offset = Number(i.value) || 0; }); draw(); };
    for (const b of $("meTab").querySelectorAll("[data-erm]")) b.onclick = () => { change("togli il bordo", () => { m.edges.splice(Number(b.dataset.erm), 1); }); tabMap(); draw(); };
    $("meEdgeAdd").onclick = () => { const to = $("meEdgeTo").value; if (!to) return; change("nuovo bordo", () => { m.edges.push({ a: l.id, side: $("meEdgeSide").value, b: to }); }); tabMap(); draw(); };
    for (const d of $("meTab").querySelectorAll("[data-pi]")) d.onclick = () => { st.sel = { type: "portal", i: Number(d.dataset.pi), end: m.portals[Number(d.dataset.pi)].a[0] === l.id ? "a" : "b" }; centerOn(Number(d.dataset.x), Number(d.dataset.y)); renderSel(); };
    $("mnAdd").onclick = () => { createMap($("mnId").value.trim(), $("mnName").value.trim(), Number($("mnW").value), Number($("mnH").value), st.secondary, $("mnUnder").checked); };
    $("mnDup").onclick = () => {
      let id = l.id + "_copia", n = 2; while (layerIdx(id) >= 0) id = l.id + "_copia" + n++;
      change("duplica la mappa", () => { m.layers.push({ ...JSON.parse(JSON.stringify({ ...l, grid: undefined })), id, name: l.name + " (copia)", grid: l.grid.map((r) => [...r]) }); st.layer = m.layers.length - 1; }, true);
      renderLayerSelect(); setLayer(st.layer);
    };
    $("mlDel").onclick = () => {
      if (m.layers.length < 2) { toast("Serve almeno una mappa", true); return; }
      const nz = zones().filter((z) => z.layer === l.id).map((z) => z.id), np = st.doc.placements.filter((p) => pl(p).pos?.layer === st.layer).length;
      if (!confirm(`Eliminare «${l.name}» con ${nz.length} zone, ${np} piazzamenti, i suoi passaggi e bordi?`)) return;
      change("elimina la mappa", () => {
        const li = st.layer;
        st.doc.placements = st.doc.placements.filter((p) => pl(p).pos?.layer !== li);
        m.zones = m.zones.filter((z) => z.layer !== l.id);
        m.portals = m.portals.filter((p) => p.a[0] !== l.id && p.b[0] !== l.id);
        m.edges = m.edges.filter((e) => e.a !== l.id && e.b !== l.id);
        for (const n of m.networks) n.zones = n.zones.filter((z) => !nz.includes(z));
        m.layers.splice(li, 1); st.layer = 0;
      }, true);
      renderLayerSelect(); setLayer(0);
    };
  }
  function renameLayer(l, id) {
    if (id === l.id) return;
    if (!/^[a-z0-9_]+$/.test(id) || layerIdx(id) >= 0 || zones().some((z) => z.id === id)) { toast("Id non valido o già usato", true); tabMap(); return; }
    const old = l.id, m = st.doc.map;
    change("rinomina la mappa", () => {
      l.id = id;
      for (const z of m.zones) if (z.layer === old) z.layer = id;
      for (const p of m.portals) for (const e of [p.a, p.b]) if (e[0] === old) e[0] = id;
      for (const e of m.edges) { if (e.a === old) e.a = id; if (e.b === old) e.b = id; }
      for (const p of st.doc.placements) { const b = pl(p).b; if (b.zone === old) b.zone = id; }
      for (const n of m.networks) n.zones = n.zones.map((z) => (z === old ? id : z));
    });
    renderLayerSelect(); tabMap();
  }
  function resizeLayer(w, h, anchor) {
    const l = L();
    const dx = [0, 0.5, 1][anchor % 3], dy = [0, 0.5, 1][Math.floor(anchor / 3)];
    return resizeTo(w, h, Math.round((w - l.width) * dx), Math.round((h - l.height) * dy), st.secondary);
  }
  /** New size of the current map; its content moves by (ox, oy) and new cells get `fill`. */
  function resizeTo(w, h, ox, oy, fill) {
    const l = L();
    if (!(w >= 4 && h >= 4 && w <= 1000 && h <= 1000)) { toast("Dimensioni fra 4 e 1000", true); return false; }
    if (w === l.width && h === l.height) return false;
    change("ridimensiona la mappa", () => {
      const g = blank(w, h, fill);
      for (let y = 0; y < l.height; y++) for (let x = 0; x < l.width; x++) if (y + oy >= 0 && y + oy < h && x + ox >= 0 && x + ox < w) g[y + oy][x + ox] = l.grid[y][x];
      l.grid = g; l.width = w; l.height = h;
      for (const z of zones()) if (z.layer === l.id) { z.rect[0] += ox; z.rect[1] += oy; }
      for (const p of st.doc.map.portals) for (const e of [p.a, p.b]) if (e[0] === l.id) { e[1] += ox; e[2] += oy; }
      for (const p of st.doc.placements) { const P = pl(p); if (P.pos?.layer === st.layer && P.b.at) P.b.at = [P.b.at[0] + ox, P.b.at[1] + oy]; }
    }, true);
    const moved = st.others.some((o) => pl(o.placement).pos?.layer === st.layer);
    if ((ox || oy) && moved) toast("Attenzione: i piazzamenti di questa mappa che stanno in altri file (col lucchetto) non vengono spostati", true);
    else toast(`Mappa ora ${w}×${h}`);
    st.view.fitted = false; fit(); buildTerrain(); renderAll();
    return true;
  }
  const closeDlg = () => { $("meDlg").style.display = "none"; };
  function openDlg(html) { $("meDlgBody").innerHTML = html; $("meDlg").style.display = "flex"; $("meDlgBody").querySelector("[data-cancel]").onclick = closeDlg; }
  function createMap(id, name, w, h, fill, under) {
    const m = st.doc.map;
    if (!/^[a-z0-9_]+$/.test(id)) { toast("Id: solo lettere minuscole, cifre e _", true); return false; }
    if (layerIdx(id) >= 0 || zones().some((z) => z.id === id)) { toast("Id già usato", true); return false; }
    if (!(w >= 4 && h >= 4 && w <= 1000 && h <= 1000)) { toast("Dimensioni fra 4 e 1000", true); return false; }
    change("nuova mappa", () => { m.layers.push({ id, name: name || id, width: w, height: h, underground: under, depth: under ? -1 : 0, tags: under ? ["sotterraneo", "dungeon"] : ["superficie"], grid: blank(w, h, fill) }); st.layer = m.layers.length - 1; }, true);
    renderLayerSelect(); setLayer(st.layer); toast("Mappa creata: collegala alle altre con un passaggio (P) o un bordo (scheda Mappa)");
    return true;
  }
  function newMapDialog() {
    const rock = st.doc.map.legend.find((t) => t.id === "roccia")?.ch ?? st.secondary, grass = st.doc.map.legend.find((t) => t.id === "erba")?.ch ?? st.secondary;
    openDlg(`<h3 style="margin:0 0 10px">Nuova mappa</h3><div class="me-form">
      Nome <input id="nmName" placeholder="es. Bunker dell'Overmind"> Id <input id="nmId" placeholder="(dal nome)">
      Tipo <select id="nmKind"><option value="under">Sotterranea (dungeon, cantina, grotta)</option><option value="surface">All'aperto (paese, campagna)</option></select>
      Dimensioni <span><input id="nmW" type="number" min="4" max="1000" value="64" style="width:80px"> × <input id="nmH" type="number" min="4" max="1000" value="40" style="width:80px"> caselle
        <button data-size="40,28">piccola</button><button data-size="64,40">media</button><button data-size="240,168">grande</button></span>
      Riempita di ${tileSelect("nmFill", rock)}</div>
      <div class="muted" style="margin-top:8px">Le mappe grandi di superficie sono 240×168. Dopo averla creata, collegala alle altre con un passaggio (strumento ⇅) o un bordo.</div>
      <div class="row" style="margin-top:10px;justify-content:flex-end"><button data-cancel>Annulla</button><button class="primary" id="nmOk">Crea</button></div>`);
    let idTouched = false;
    $("nmName").oninput = () => { if (!idTouched) $("nmId").value = slug($("nmName").value); };
    $("nmId").oninput = () => { idTouched = true; };
    $("nmKind").onchange = () => { $("nmFill").value = $("nmKind").value === "under" ? rock : grass; };
    for (const b of $("meDlgBody").querySelectorAll("[data-size]")) b.onclick = () => { [$("nmW").value, $("nmH").value] = b.dataset.size.split(","); };
    $("nmOk").onclick = () => { if (createMap($("nmId").value.trim() || slug($("nmName").value), $("nmName").value.trim(), Number($("nmW").value), Number($("nmH").value), $("nmFill").value, $("nmKind").value === "under")) closeDlg(); };
    $("nmName").focus();
  }
  function growDialog() {
    const l = L();
    const side = (k, t) => `<span style="white-space:nowrap">${t} <input id="gr${k}" type="number" value="0" style="width:70px"> <button data-add="${k},10">+10</button><button data-add="${k},40">+40</button></span>`;
    openDlg(`<h3 style="margin:0 0 4px">Amplia «${esc(l.name)}»</h3><div class="muted" style="margin-bottom:10px">Ora è ${l.width}×${l.height}. Scrivi quante caselle aggiungere su ogni lato (un numero negativo le toglie).</div>
      <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;align-items:center;justify-items:center">
        <span></span>${side("N", "Nord")}<span></span>
        ${side("W", "Ovest")}<div id="grPrev" class="card" style="margin:0;text-align:center;min-width:110px"></div>${side("E", "Est")}
        <span></span>${side("S", "Sud")}<span></span></div>
      <div class="me-form" style="margin-top:10px">Caselle nuove ${tileSelect("grFill", st.secondary)}</div>
      <div class="muted" style="margin-top:8px">Zone, passaggi, edifici e NPC di questa mappa si spostano insieme al contenuto. Se la mappa confina con un'altra lungo un bordo, ricontrolla lo «spostamento» del bordo nella scheda Mappa.</div>
      <div class="row" style="margin-top:10px;justify-content:flex-end"><button data-cancel>Annulla</button><button class="primary" id="grOk">Amplia</button></div>`);
    const val = (k) => Math.round(Number($("gr" + k).value) || 0);
    const prev = () => { $("grPrev").innerHTML = `<b>${l.width + val("W") + val("E")} × ${l.height + val("N") + val("S")}</b><div class="muted">nuove dimensioni</div>`; };
    for (const k of "NSEW") $("gr" + k).oninput = prev;
    for (const b of $("meDlgBody").querySelectorAll("[data-add]")) b.onclick = () => { const [k, n] = b.dataset.add.split(","); $("gr" + k).value = val(k) + Number(n); prev(); };
    prev();
    $("grOk").onclick = () => {
      if (!(val("N") || val("S") || val("E") || val("W"))) { closeDlg(); return; }
      if (resizeTo(l.width + val("W") + val("E"), l.height + val("N") + val("S"), val("W"), val("N"), $("grFill").value)) closeDlg();
    };
  }
  function tabLegend() {
    const m = st.doc.map, counts = {};
    for (const l of m.layers) for (const r of l.grid) for (const c of r) counts[c] = (counts[c] ?? 0) + 1;
    const sprites = [...S.assets].filter((f) => f.startsWith("tile_") && !f.endsWith("_face.png")).map((f) => f.slice(5, -4)).sort();
    const groups = [...new Set([...GROUP_ORDER, ...m.legend.map((t) => t.group).filter(Boolean)])];
    $("meTab").innerHTML = `<div class="muted">La legenda è aperta in una finestra.</div><div class="row" style="margin-top:6px"><button id="meLegOpen">Riapri la legenda</button></div>`;
    $("meLegOpen").onclick = tabLegend;
    const box = $("meLegBody");
    $("meLegDlg").style.display = "flex";
    box.innerHTML = `<div class="muted">Ogni carattere delle mappe è un tipo di casella. Il colore tinge la figura; «figura» sceglie l'immagine tile_*.png.</div>
      <datalist id="meGroups">${groups.map((g) => `<option value="${esc(g)}">`).join("")}</datalist>
      <div class="scrollbox" style="max-height:62vh"><table class="me-leg"><tr><th></th><th>car.</th><th>nome / id</th><th>passa</th><th>gruppo</th><th>figura e colore</th><th>scavo</th><th>usi</th><th></th></tr>
      ${m.legend.map((t, i) => `<tr><td><img class="me-t s" src="${artUrl(t)}" alt=""></td><td><code>${esc(t.ch)}</code></td>
        <td><input data-i="${i}" data-f="name" value="${esc(t.name ?? "")}" size="20"><br><input data-i="${i}" data-f="id" value="${esc(t.id)}" size="20" class="muted"></td>
        <td><input type="checkbox" data-i="${i}" data-f="walkable"${t.walkable !== false ? " checked" : ""}></td>
        <td><input data-i="${i}" data-f="group" value="${esc(t.group ?? "")}" list="meGroups" size="14"></td>
        <td style="white-space:nowrap"><select data-i="${i}" data-f="sprite">${opts(sprites.map((s) => [s, s]), t.sprite ?? "", "(" + t.id + ")")}</select> <input type="color" data-i="${i}" data-f="color" value="${t.color ?? "#ffffff"}"> <button class="x" data-nocolor="${i}" title="senza colore"${t.color ? "" : " disabled"}>✕</button></td>
        <td><select data-i="${i}" data-f="dig_to">${opts(m.legend.map((x) => [x.ch, x.name || x.id]), t.dig_to ?? "", "no")}</select>${t.dig_to ? `<br><select data-i="${i}" data-yi>${opts(Object.values(C("items")).map((x) => [x.id, x.name]), t.yields?.[0] ?? "", "niente")}</select><input class="num" style="width:40px" data-i="${i}" data-yq value="${t.yields?.[1] ?? 1}">` : ""}</td>
        <td class="num">${counts[t.ch] ?? 0}</td><td>${counts[t.ch] ? "" : `<button class="x" data-del="${i}" title="elimina (non è usata)">✕</button>`}</td></tr>`).join("")}</table></div>
      <div class="row" style="margin-top:6px"><button id="meLegAdd">Nuovo tipo di casella</button> <span class="muted">Le caselle colorate si creano più comodamente dalla Tavolozza.</span></div>`;
    const re = () => { rebuildLegend(); for (const k of Object.keys(arts)) delete arts[k]; buildTerrain(); tabLegend(); renderToolBox(); };
    for (const el of box.querySelectorAll("[data-f]")) el.onchange = () => {
      const t = m.legend[Number(el.dataset.i)], f = el.dataset.f;
      if (f === "id") { const v = slug(el.value); if (!v || m.legend.some((x) => x !== t && x.id === v)) { toast("Id non valido o già usato", true); tabLegend(); return; } }
      change("legenda", () => {
        const v = el.type === "checkbox" ? el.checked : el.value;
        if (f === "walkable") t.walkable = v;
        else if (f === "id") t.id = slug(v);
        else if (v === "") delete t[f];
        else t[f] = v;
        if (f === "dig_to" && !v) delete t.yields;
      });
      re();
    };
    for (const b of box.querySelectorAll("[data-nocolor]")) b.onclick = () => { change("legenda", () => { delete m.legend[Number(b.dataset.nocolor)].color; }); re(); };
    for (const el of box.querySelectorAll("[data-yi],[data-yq]")) el.onchange = () => {
      const t = m.legend[Number(el.dataset.i)], row = el.closest("td");
      const item = row.querySelector("[data-yi]").value, q = Math.max(1, Number(row.querySelector("[data-yq]").value) || 1);
      change("legenda", () => { if (item) t.yields = [item, q]; else delete t.yields; }); re();
    };
    for (const b of box.querySelectorAll("[data-del]")) b.onclick = () => { change("elimina casella", () => { m.legend.splice(Number(b.dataset.del), 1); }); re(); };
    $("meLegAdd").onclick = () => {
      const ch = freeChar(); if (!ch) return;
      let id = "casella", n = 2; while (m.legend.some((x) => x.id === id)) id = "casella_" + n++;
      change("nuova casella", () => { m.legend.push({ ch, id, name: "Nuova casella", walkable: true, sprite: "piastrelle", group: "Altro" }); }); re();
    };
  }

  // ── Checks ──────────────────────────────────────────────────────────────────
  function runCheck() {
    const out = [], m = st.doc.map;
    const at = (li, x, y) => ({ li, x, y });
    const solidAt = (li, x, y) => { const l = m.layers[li]; return !l || x < 0 || y < 0 || x >= l.width || y >= l.height ? "fuori dalla mappa" : !walkable(l.grid[y][x]) ? `su una casella solida (${tileName(l.grid[y][x])})` : null; };
    st.doc.placements.forEach((p, i) => {
      const P = pl(p);
      if (P.kind === "Player") return;
      const what = P.kind === "Entity" ? "NPC " + (P.b.name || P.b.template) : P.kind === "Hidden" ? "Oggetto nascosto" : "Edificio " + (P.b.name || P.b.building);
      if (!P.pos) { out.push({ bad: true, text: `${what}: la zona '${P.b.zone}' non esiste` }); return; }
      if (P.b.at) {
        const s = solidAt(P.pos.layer, P.pos.x, P.pos.y);
        if (s) out.push({ bad: s.startsWith("fuori"), text: `${what} ${s}`, go: at(P.pos.layer, P.pos.x, P.pos.y), sel: { type: "pl", i } });
        else if (!inZone(P.b.zone, P.pos.layer, P.pos.x, P.pos.y)) out.push({ text: `${what} è fuori dalla sua zona (${P.b.zone})`, go: at(P.pos.layer, P.pos.x, P.pos.y), sel: { type: "pl", i } });
      }
      if (P.kind === "Hidden" && !Object.values(P.b.stock ?? {}).some((n) => n > 0)) out.push({ text: "Oggetto nascosto vuoto", go: at(P.pos.layer, P.pos.x, P.pos.y), sel: { type: "pl", i } });
    });
    m.portals.forEach((p, i) => { for (const k of ["a", "b"]) { const e = p[k], li = layerIdx(e[0]); if (li < 0) { out.push({ bad: true, text: `Passaggio ${p.name}: la mappa '${e[0]}' non esiste` }); continue; } const s = solidAt(li, e[1], e[2]); if (s) out.push({ bad: s.startsWith("fuori"), text: `Passaggio ${p.name} (${k.toUpperCase()}) ${s}`, go: at(li, e[1], e[2]), sel: { type: "portal", i, end: k } }); } });
    for (const z of m.zones) {
      const li = layerIdx(z.layer), l = m.layers[li]; if (!l) { out.push({ bad: true, text: `Zona ${z.name}: la mappa '${z.layer}' non esiste` }); continue; }
      const [x, y, w, h] = z.rect;
      if (x < 0 || y < 0 || x + w > l.width || y + h > l.height) out.push({ text: `Zona ${z.name} esce dalla mappa`, go: at(li, x, y), sel: { type: "zone", id: z.id } });
    }
    for (const l of m.layers) if (!m.portals.some((p) => p.a[0] === l.id || p.b[0] === l.id) && !m.edges.some((e) => e.a === l.id || e.b === l.id)) out.push({ text: `La mappa ${l.name} non è collegata a niente: non ci si arriva` });
    out.push({ text: "Chiedo al server di verificare tutti i contenuti con questa mappa…", pending: true });
    st.tab = "check"; showCheck(out, false);
    save(false, true);
  }
  function showCheck(items, fromServer) {
    if (fromServer) items = [...(st.lastCheck ?? []).filter((x) => !x.pending && !x.server), ...items.map((x) => ({ ...x, server: true }))];
    st.lastCheck = items;
    if (st.tab !== "check") { st.tab = "check"; }
    for (const b of $("meTabs").querySelectorAll("button")) b.classList.toggle("on", b.dataset.tab === "check");
    const bad = items.filter((x) => x.bad).length, warn = items.filter((x) => !x.bad && !x.ok && !x.pending).length;
    $("meTab").innerHTML = `<div class="row"><button id="meReCheck">Verifica di nuovo</button> <span class="${bad ? "bad" : "ok"}">${bad} errori</span> <span class="muted">${warn} avvisi</span></div><div class="me-check">` +
      (items.map((x, i) => `<div data-c="${i}" class="${x.bad ? "bad" : x.ok ? "ok" : x.pending ? "muted" : ""}">${x.bad ? "✖" : x.ok ? "✔" : x.pending ? "…" : "⚠"} ${esc(x.text)}${x.server ? ' <span class="muted">(server)</span>' : ""}</div>`).join("") || '<div class="muted">Premi Verifica.</div>') + `</div>`;
    $("meReCheck").onclick = runCheck;
    for (const d of $("meTab").querySelectorAll("[data-c]")) d.onclick = () => {
      const x = items[Number(d.dataset.c)]; if (!x.go) return;
      if (x.go.li !== st.layer) setLayer(x.go.li);
      if (x.sel) { st.sel = x.sel; renderSel(); }
      centerOn(x.go.x, x.go.y);
    };
  }

  function renderStatus() {
    if (!st.doc) return;
    $("meDirty").innerHTML = st.dirty ? '<span class="me-dirty">● modifiche non salvate</span>' : '<span class="muted">salvato</span>';
    $("meUndo").disabled = !st.undo.length; $("meRedo").disabled = !st.redo.length;
    $("meUndo").title = st.undo.length ? "Annulla: " + st.undo[st.undo.length - 1].label + " (Ctrl+Z)" : "Annulla (Ctrl+Z)";
    const h = st.hover, l = L();
    let s = `<span>File <b>${esc(st.file)}</b></span><span>Mappa <b>${esc(l.name)}</b> ${l.width}×${l.height}</span>`;
    if (h && l.grid[h.y]?.[h.x] != null) {
      const ch = l.grid[h.y][h.x], t = st.leg[ch];
      const zs = zones().filter((z) => z.layer === l.id && h.x >= z.rect[0] && h.y >= z.rect[1] && h.x < z.rect[0] + z.rect[2] && h.y < z.rect[1] + z.rect[3]).map((z) => z.name);
      s += `<span>(<b>${h.x}</b>, <b>${h.y}</b>)</span><span>${tileImgHtml(ch)} <b>${esc(t?.name ?? ch)}</b> '${esc(ch)}' ${t?.walkable === false ? "· solida" : ""}</span><span>${esc(zs.join(" · ") || "nessuna zona")}</span>`;
    }
    if (st.drag?.a && st.drag.b) s += `<span>${Math.abs(st.drag.a.x - st.drag.b.x) + 1}×${Math.abs(st.drag.a.y - st.drag.b.y) + 1}</span>`;
    $("meStatus").innerHTML = s;
  }

  async function open() {
    buildUi();
    if (!st.doc) {
      try { await load(true); } catch (e) { $("meTab").innerHTML = `<div class="bad">${esc(e.message)}</div>`; toast(e.message, true); return; }
    }
    resize(); renderAll();
  }
  return { open, state: st };
})();
