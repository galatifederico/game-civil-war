// Admin web app: static page, no build step. It talks to the backend it is served from
// (/auth/login, /worlds, /admin/api/...). Everything shown comes from the server; nothing is
// decided here except how to lay it out. All DOM is built with textContent (never innerHTML with
// data), so names typed by players cannot inject markup.
'use strict';

const $ = (id) => document.getElementById(id);
const TOKEN_KEY = 'thegame.admin.token';
const WORLD_KEY = 'thegame.admin.world';
const USER_KEY = 'thegame.admin.user';
const SIDE_KEY = 'thegame.admin.sideCollapsed';

const state = {
  token: null,
  username: '',
  worlds: [],
  worldId: null,
  def: null,      // the world as the server has it
  draft: {},      // list name -> working copy being edited
  rulesDraft: null,
  tab: 'world',
  selected: null, // {list, index}: the row the map places
  view: null,     // a detail page inside a section: {type: 'race'|'char', id|key, draft}
};

// --- helpers ---

function el(tag, props, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(props || {})) {
    if (k === 'class') node.className = v;
    else if (k.startsWith('on')) node.addEventListener(k.slice(2), v);
    else if (v !== undefined && v !== null && v !== false) node.setAttribute(k, v === true ? '' : v);
  }
  for (const c of children.flat()) {
    if (c === null || c === undefined || c === false) continue;
    node.append(c.nodeType ? c : document.createTextNode(String(c)));
  }
  return node;
}

function notice(text, kind) {
  const n = $('notice');
  n.textContent = text || '';
  n.className = kind || '';
}

async function api(method, path, body) {
  const res = await fetch(path, {
    method,
    headers: { 'Content-Type': 'application/json', ...(state.token ? { Authorization: 'Bearer ' + state.token } : {}) },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  let data = {};
  try { data = await res.json(); } catch (_) { /* empty body */ }
  if (res.status === 401 && state.token) {
    logout();
    throw new Error(data.error || 'Sessione scaduta');
  }
  if (!res.ok) throw new Error(data.error || 'Errore ' + res.status);
  return data;
}

const clone = (v) => JSON.parse(JSON.stringify(v));

// "soldi=5, alcol=20" <-> {soldi: 5, alcol: 20}
function traitsToText(obj) {
  return Object.entries(obj || {}).map(([k, v]) => k + '=' + v).join(', ');
}
function textToTraits(text) {
  const out = {};
  for (const part of text.split(',')) {
    const s = part.trim();
    if (!s) continue;
    const [k, v] = s.split('=').map((x) => x.trim());
    const n = Number(v);
    if (!k || v === undefined || v === '' || !Number.isFinite(n)) throw new Error('Caratteristiche non valide: "' + s + '" (usa nome=valore, separati da virgole)');
    out[k] = n;
  }
  return out;
}

function getPath(obj, path) { return path.split('.').reduce((o, k) => (o == null ? o : o[k]), obj); }
function setPath(obj, path, value) {
  const keys = path.split('.');
  const last = keys.pop();
  const target = keys.reduce((o, k) => (o[k] = o[k] || {}), obj);
  target[last] = value;
}

// --- session ---

function logout() {
  state.token = null;
  try { sessionStorage.removeItem(TOKEN_KEY); } catch (_) { /* storage may be blocked */ }
  show('login');
}

function show(which) {
  $('login').hidden = which !== 'login';
  $('editor').hidden = which !== 'editor';
  $('top').hidden = which === 'login';
}

async function doLogin() {
  try {
    const out = await api('POST', '/auth/login', { email: $('email').value, password: $('password').value });
    state.token = out.token;
    state.username = out.username;
    try { sessionStorage.setItem(TOKEN_KEY, out.token); sessionStorage.setItem(USER_KEY, out.username); } catch (_) { /* ignore */ }
    $('password').value = '';
    await start();
  } catch (e) {
    alert(e.message);
  }
}

async function start() {
  $('who').textContent = state.username;
  const out = await api('GET', '/worlds');
  state.worlds = out.worlds.filter((w) => w.admin);
  const picker = $('worldPicker');
  picker.replaceChildren(...state.worlds.map((w) => el('option', { value: w.id }, w.name)));
  show('editor');
  if (state.worlds.length === 0) {
    $('tabs').replaceChildren();
    $('panel').replaceChildren(el('p', { class: 'muted' }, 'Non sei admin di nessun mondo. Creane uno con "Nuovo mondo".'));
    return;
  }
  let saved = null;
  try { saved = localStorage.getItem(WORLD_KEY); } catch (_) { /* ignore */ }
  state.worldId = state.worlds.some((w) => w.id === saved) ? saved : state.worlds[0].id;
  picker.value = state.worldId;
  await loadWorld();
}

async function loadWorld() {
  try { localStorage.setItem(WORLD_KEY, state.worldId); } catch (_) { /* ignore */ }
  state.def = await api('GET', '/admin/api/worlds/' + state.worldId);
  state.draft = {};
  state.rulesDraft = clone(state.def.effective_rules);
  state.selected = null;
  renderTabs();
  renderPanel();
}

// --- tabs ---

const TABS = [
  ['world', 'Mondo', '◉'],
  ['rules', 'Regole', '⚙'],
  ['characteristics', 'Caratteristiche', '≡'],
  ['races', 'Razze', '☺'],
  ['compat', 'Compatibilità', '⚭'],
  ['boards', 'Board', '▦'],
  ['terrain', 'Terreno', '▤'],
  ['links', 'Passaggi', '⇄'],
  ['goals', 'Obiettivi', '★'],
  ['npcs', 'NPC', '☻'],
  ['items', 'Oggetti', '◆'],
  ['players', 'Giocatori', '♟'],
];

function renderTabs() {
  $('tabs').replaceChildren(...TABS.map(([id, label, icon]) =>
    el('button', {
      class: id === state.tab ? 'active' : '', title: label,
      onclick: () => { state.tab = id; state.view = null; state.selected = null; notice(''); renderTabs(); renderPanel(); },
    }, el('span', { class: 'ico' }, icon), el('span', { class: 'label' }, label))));
}

// Il menu laterale si comprime (resta solo l'icona di ogni sezione); la scelta si ricorda.
function setSideCollapsed(collapsed) {
  $('side').classList.toggle('collapsed', collapsed);
  $('sideToggle').textContent = collapsed ? '»' : '«';
  try { localStorage.setItem(SIDE_KEY, collapsed ? '1' : '0'); } catch (_) { /* ignore */ }
}

function renderPanel() {
  const panel = $('panel');
  const renderer = {
    world: renderWorld, rules: renderRules, players: renderPlayers, terrain: renderTerrain,
    races: () => (state.view && state.view.type === 'race' ? renderRaceDetail() : renderRaceList()),
    characteristics: () => (state.view && state.view.type === 'char' ? renderCharDetail() : renderCharList()),
  }[state.tab];
  panel.replaceChildren(...(renderer ? renderer() : [renderList(state.tab)]));
  drawMap(); // no-op unless this tab has a map
  drawTerrain(); // same for the terrain painter
}

// Saving restarts the world so that it picks the change up: players reconnect on their own.
async function save(path, body) {
  notice('Salvataggio...');
  try {
    await api('PUT', '/admin/api/worlds/' + state.worldId + path, body);
  } catch (e) {
    notice(e.message, 'bad');
    return false;
  }
  const tab = state.tab;
  await loadWorld();
  state.tab = tab;
  renderTabs();
  renderPanel();
  notice('Salvato. Il mondo è stato riavviato con le modifiche.', 'ok');
  return true;
}

// --- world tab ---

function renderWorld() {
  const name = el('input', { type: 'text', value: state.def.name, maxlength: 40 });
  const description = el('textarea', { rows: 3, maxlength: 500 });
  description.value = state.def.description;
  return [
    el('div', { class: 'card', style: 'display:flex;flex-direction:column;gap:12px;max-width:640px' },
      el('h2', {}, 'Mondo'),
      el('label', {}, 'Nome', name),
      el('label', {}, 'Descrizione (la vedono i giocatori nella lobby)', description),
      el('div', { class: 'actions' },
        el('button', { onclick: () => save('/meta', { name: name.value, description: description.value }) }, 'Salva')),
      el('p', { class: 'muted' }, 'Ogni salvataggio riavvia il mondo: i giocatori connessi vengono scollegati e si riconnettono da soli, senza perdere i progressi.'),
      el('p', { class: 'muted' }, 'ID: ' + state.def.id)),
  ];
}

// --- rules tab: one input per rule, laid out from the structure the server sends ---

function renderRules() {
  const defaults = state.def.default_rules;
  const draft = state.rulesDraft;
  const groups = [];
  const top = el('div', {});
  const build = (obj, base, path, container) => {
    for (const [key, value] of Object.entries(obj)) {
      if (path.length === 0 && (key === 'trait_names' || key === 'characteristics')) continue; // si gestiscono in Caratteristiche
      const full = path.concat(key);
      const label = full.join('.');
      if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
        const inner = el('div', {});
        groups.push(el('fieldset', {}, el('legend', {}, label), inner));
        build(value, base[key], full, inner);
        continue;
      }
      const changed = () => JSON.stringify(getPath(draft, label)) !== JSON.stringify(getPath(defaults, label));
      let input;
      if (typeof value === 'boolean') {
        input = el('input', { type: 'checkbox', onchange: (e) => { setPath(draft, label, e.target.checked); mark(); } });
        input.checked = value;
      } else if (typeof value === 'number') {
        input = el('input', { type: 'number', value: value, onchange: (e) => { setPath(draft, label, Number(e.target.value)); mark(); } });
      } else if (Array.isArray(value)) {
        input = el('input', { type: 'text', value: value.join(', '), onchange: (e) => { setPath(draft, label, e.target.value.split(',').map((s) => s.trim()).filter(Boolean)); mark(); } });
      } else if (label === 'goal_assignment') {
        input = el('select', { onchange: (e) => { setPath(draft, label, e.target.value); mark(); } },
          el('option', { value: 'random' }, 'random (a caso)'), el('option', { value: 'manual' }, 'manual (li assegni tu)'));
        input.value = value;
      } else {
        input = el('input', { type: 'text', value: value, onchange: (e) => { setPath(draft, label, e.target.value); mark(); } });
      }
      const mark = () => input.classList.toggle('changed', changed());
      mark();
      const box = typeof value === 'boolean'
        ? el('label', { class: 'check' }, input, key)
        : el('label', {}, key, input);
      container.append(box);
    }
  };
  build(draft, defaults, [], top);
  // The plain values come first, then the groups.
  return [
    el('h2', {}, 'Regole del mondo'),
    el('p', { class: 'muted' }, 'I nomi delle caratteristiche estese e i loro limiti si modificano nella sezione Caratteristiche. Le caselle con il bordo giallo differiscono dai valori predefiniti. Si salvano solo le differenze: se un giorno cambia un valore predefinito, i mondi che non lo hanno modificato lo seguono.'),
    el('div', { class: 'rules' }, el('fieldset', {}, el('legend', {}, 'generali'), top), ...groups),
    el('div', { class: 'actions' },
      el('button', { onclick: () => save('/rules', state.rulesDraft) }, 'Salva regole'),
      el('button', { class: 'ghost', onclick: () => { state.rulesDraft = clone(state.def.effective_rules); renderPanel(); notice(''); } }, 'Annulla modifiche'),
      el('button', { class: 'ghost', onclick: () => { state.rulesDraft = clone(defaults); renderPanel(); } }, 'Ripristina i predefiniti')),
  ];
}

// --- generic list editor ---

const boardOptions = () => state.def.boards.map((b) => [b.id, b.name]);
const raceOptions = (withNone) => (withNone ? [['', '(nessuna)']] : []).concat(state.def.races.map((r) => [r.id, r.name]));

// Each list: how to edit it. kinds: text, area, number, select, traits.
const LISTS = {
  boards: {
    title: 'Board', path: '/boards', blank: () => ({ id: '', name: 'Nuova board', width: 16, height: 16, grid: 'square' }),
    help: 'Ogni board è una zona del mondo, a caselle quadrate. Una board non si può cancellare se ci sono pedine o strutture dei giocatori; se la rimpicciolisci, niente deve restare fuori.',
    cols: [
      { key: 'name', label: 'Nome', kind: 'text' },
      { key: 'width', label: 'Larghezza', kind: 'number' },
      { key: 'height', label: 'Altezza', kind: 'number' },
    ],
  },
  links: {
    title: 'Passaggi', path: '/links', blank: () => ({ from_board: state.def.boards[0].id, from_x: 0, from_y: 0, to_board: state.def.boards[0].id, to_x: 1, to_y: 1 }),
    help: 'Un passaggio porta chi ci mette piede su un\'altra casella, di solito di un\'altra board. Vale in un solo verso: per andare e tornare servono due passaggi.',
    map: 'links',
    cols: [
      { key: 'from_board', label: 'Da board', kind: 'select', options: boardOptions },
      { key: 'from_x', label: 'x', kind: 'number' },
      { key: 'from_y', label: 'y', kind: 'number' },
      { key: 'to_board', label: 'A board', kind: 'select', options: boardOptions },
      { key: 'to_x', label: 'x', kind: 'number' },
      { key: 'to_y', label: 'y', kind: 'number' },
    ],
  },
  compat: {
    title: 'Compatibilità', path: '/compat', blank: () => ({ race_a: state.def.races[0]?.id, race_b: state.def.races[0]?.id, child_race: state.def.races[0]?.id }),
    help: 'Quali coppie di razze possono avere figli e di che razza nascono. Vale per la coppia in qualunque ordine (anche due della stessa razza).',
    cols: [
      { key: 'race_a', label: 'Razza A', kind: 'select', options: () => raceOptions(false) },
      { key: 'race_b', label: 'Razza B', kind: 'select', options: () => raceOptions(false) },
      { key: 'child_race', label: 'Figlio', kind: 'select', options: () => raceOptions(false) },
    ],
  },
  goals: {
    title: 'Obiettivi', path: '/goals', blank: () => ({ id: '', scope: 'individual', kind: 'points', target: 10, reward: 10, title: 'Nuovo obiettivo', description: '', achieved_by: '' }),
    help: '"world": vince chi lo raggiunge per primo. "individual": ne segue uno alla volta ogni giocatore (assegnato a caso o da te, secondo le regole). Il premio è in punti squadra.',
    cols: [
      { key: 'title', label: 'Titolo', kind: 'text' },
      { key: 'description', label: 'Descrizione', kind: 'area' },
      { key: 'scope', label: 'Ambito', kind: 'select', options: () => [['individual', 'individual'], ['world', 'world']] },
      { key: 'kind', label: 'Misura', kind: 'select', options: () => state.def.goal_kinds.map((k) => [k, k]) },
      { key: 'target', label: 'Traguardo', kind: 'number' },
      { key: 'reward', label: 'Premio', kind: 'number' },
    ],
  },
  npcs: {
    title: 'NPC', path: '/npcs', map: 'entities',
    blank: () => ({ id: '', board_id: state.def.boards[0].id, name: 'Nuovo NPC', description: '', x: 0, y: 0, speed: 0, health: 100, vision: 3, strength: 10, dialogue: '', race_id: '', traits: {}, sprite: '' }),
    help: 'Gli NPC non li controlla nessuno. Il dialogo ha una battuta per riga: ne risponde una a caso a ogni conversazione. Seleziona una riga e clicca sulla mappa per spostarla.',
    cols: [
      { key: 'name', label: 'Nome', kind: 'text' },
      { key: 'description', label: 'Descrizione', kind: 'area' },
      { key: 'board_id', label: 'Board', kind: 'select', options: boardOptions },
      { key: 'x', label: 'x', kind: 'number' }, { key: 'y', label: 'y', kind: 'number' },
      { key: 'health', label: 'Vita', kind: 'number' }, { key: 'strength', label: 'Forza', kind: 'number' },
      { key: 'speed', label: 'Velocità', kind: 'number' }, { key: 'vision', label: 'Vista', kind: 'number' },
      { key: 'dialogue', label: 'Dialogo (una battuta per riga)', kind: 'area' },
      { key: 'race_id', label: 'Razza', kind: 'select', options: () => raceOptions(true) },
      { key: 'traits', label: 'Caratteristiche', kind: 'traits' },
      { key: 'sprite', label: 'Sprite', kind: 'select', options: () => [['', '(predefinito)']].concat(state.def.sprites.map((k) => [k, k])) },
    ],
  },
  items: {
    title: 'Oggetti', path: '/items', map: 'entities',
    blank: () => ({ id: '', board_id: state.def.boards[0].id, name: 'Nuovo oggetto', description: '', x: 0, y: 0, effect: {}, icon: '' }),
    help: 'Oggetti a terra. Chi li raccoglie li mette nell\'inventario della squadra; il champion può usarli e l\'effetto (cura, punti, forza, caratteristiche) si applica a lui. Con tutti gli effetti a zero l\'oggetto è solo decorativo.',
    cols: [
      { key: 'name', label: 'Nome', kind: 'text' },
      { key: 'description', label: 'Descrizione', kind: 'area' },
      { key: 'board_id', label: 'Board', kind: 'select', options: boardOptions },
      { key: 'x', label: 'x', kind: 'number' }, { key: 'y', label: 'y', kind: 'number' },
      { key: 'effect.heal', label: 'Cura', kind: 'number' },
      { key: 'effect.points', label: 'Punti', kind: 'number' },
      { key: 'effect.strength', label: 'Forza', kind: 'number' },
      { key: 'effect.traits', label: 'Caratteristiche', kind: 'traits' },
      { key: 'icon', label: 'Icona (inventario)', kind: 'select', options: () => [['', '(automatica)']].concat(state.def.item_icons.map((k) => [k, k])) },
    ],
  },
};

function cellInput(list, cols, row, col, index) {
  const value = getPath(row, col.key);
  const set = (v) => setPath(row, col.key, v);
  switch (col.kind) {
    case 'number':
      return el('input', { type: 'number', value: value ?? 0, onchange: (e) => { set(Number(e.target.value)); if (list.map) drawMap(); } });
    case 'select': {
      const select = el('select', { onchange: (e) => { set(e.target.value); if (list.map) drawMap(); } },
        ...col.options().map(([v, label]) => el('option', { value: v }, label)));
      select.value = value ?? '';
      return select;
    }
    case 'area': {
      const area = el('textarea', { onchange: (e) => set(e.target.value) });
      area.value = value ?? '';
      return area;
    }
    case 'traits': {
      const input = el('input', { type: 'text', value: traitsToText(value), placeholder: 'nome=valore, ...' });
      input.addEventListener('change', () => {
        try { set(textToTraits(input.value)); input.style.borderColor = ''; notice(''); }
        catch (e) { input.style.borderColor = 'var(--bad)'; notice(e.message, 'bad'); }
      });
      return input;
    }
    default:
      return el('input', { type: 'text', value: value ?? '', onchange: (e) => set(e.target.value) });
  }
}

function renderList(name) {
  const list = LISTS[name];
  if (!state.draft[name]) state.draft[name] = clone(state.def[name]);
  const rows = state.draft[name];

  const head = el('tr', {}, ...list.cols.map((c) => el('th', {}, c.label)), el('th', {}));
  const body = rows.map((row, i) => el('tr',
    {
      class: state.selected && state.selected.list === name && state.selected.index === i ? 'selected' : '',
      onfocusin: () => selectRow(name, i),
    },
    ...list.cols.map((c) => el('td', {}, cellInput(list, list.cols, row, c, i))),
    el('td', {}, el('button', { class: 'danger', title: 'Elimina la riga', onclick: () => { rows.splice(i, 1); state.selected = null; renderPanel(); } }, '✕'))));

  const parts = [
    el('h2', {}, list.title + ' (' + rows.length + ')'),
    el('p', { class: 'muted' }, list.help),
    el('div', { class: 'tablewrap' }, el('table', {}, el('thead', {}, head), el('tbody', {}, body))),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { rows.push(list.blank()); selectRow(name, rows.length - 1, true); } }, '+ Aggiungi'),
      el('button', { onclick: () => save(list.path, { items: rows }) }, 'Salva ' + list.title),
      el('button', { class: 'ghost', onclick: () => { delete state.draft[name]; renderPanel(); notice(''); } }, 'Annulla modifiche')),
  ];
  if (list.map) parts.push(mapBox(name));
  return el('div', {}, ...parts);
}

function selectRow(list, index, rerender) {
  const same = state.selected && state.selected.list === list && state.selected.index === index;
  state.selected = { list, index };
  if (rerender) { renderPanel(); return; }
  if (!same) {
    // Mark the row without rebuilding the table (that would drop the focus).
    document.querySelectorAll('#panel tbody tr').forEach((tr, i) => tr.classList.toggle('selected', i === index));
    if (LISTS[list].map) drawMap();
  }
}

// --- map: shows the cells of a board with what is on them; clicking places the selected row ---

const mapView = { boardId: null, cell: 18 };

function mapBox(listName) {
  const canvas = el('canvas', { id: 'map' });
  canvas.addEventListener('click', (e) => clickMap(e, canvas));
  return el('div', { class: 'mapbox' },
    el('h3', {}, 'Mappa'),
    el('div', { class: 'legend' },
      legend('#e0c060', 'NPC'), legend('#6fb3ff', 'oggetto'), legend('#c084fc', 'passaggio'),
      legend('#e8eaf0', 'riga selezionata')),
    el('label', { style: 'max-width:240px' }, 'Board mostrata',
      el('select', { id: 'mapBoard', onchange: (e) => { mapView.boardId = e.target.value; drawMap(); } },
        ...state.def.boards.map((b) => el('option', { value: b.id }, b.name)))),
    canvas);
}

function legend(color, text) {
  const dot = el('span', { class: 'dot' });
  dot.style.background = color;
  return el('span', {}, dot, text);
}

function currentBoard() {
  const list = state.selected && LISTS[state.selected.list].map ? state.draft[state.selected.list] : null;
  const row = list && list[state.selected.index];
  const rowBoard = row && (row.board_id || row.from_board);
  const id = rowBoard || mapView.boardId || state.def.boards[0].id;
  mapView.boardId = state.def.boards.some((b) => b.id === id) ? id : state.def.boards[0].id;
  return state.def.boards.find((b) => b.id === mapView.boardId);
}

// Cell centre on the canvas: plain rows and columns.
function cellCenter(board, x, y) {
  const c = mapView.cell;
  return [x * c + c / 2, y * c + c / 2];
}

function drawMap() {
  const canvas = $('map');
  if (!canvas) return;
  const board = currentBoard();
  const select = $('mapBoard');
  if (select) select.value = board.id;
  const c = mapView.cell;
  canvas.width = board.width * c + 1;
  canvas.height = board.height * c + 1;
  const ctx = canvas.getContext('2d');
  ctx.strokeStyle = '#262a36';
  for (let y = 0; y < board.height; y++) {
    for (let x = 0; x < board.width; x++) {
      const [cx, cy] = cellCenter(board, x, y);
      ctx.strokeRect(cx - c / 2 + 0.5, cy - c / 2 + 0.5, c - 1, c - 1);
    }
  }
  const mark = (x, y, color, ring) => {
    const [cx, cy] = cellCenter(board, x, y);
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(cx, cy, c / 3, 0, Math.PI * 2);
    ctx.fill();
    if (ring) { ctx.strokeStyle = '#fff'; ctx.lineWidth = 2; ctx.stroke(); ctx.lineWidth = 1; }
  };
  const sel = state.selected;
  const selected = (list, i) => sel && sel.list === list && sel.index === i;
  (state.draft.npcs || state.def.npcs).forEach((n, i) => { if (n.board_id === board.id) mark(n.x, n.y, '#e0c060', selected('npcs', i)); });
  (state.draft.items || state.def.items).forEach((n, i) => { if (n.board_id === board.id) mark(n.x, n.y, '#6fb3ff', selected('items', i)); });
  (state.draft.links || state.def.links).forEach((l, i) => {
    if (l.from_board === board.id) mark(l.from_x, l.from_y, '#c084fc', selected('links', i));
  });
}

function clickMap(e, canvas) {
  const sel = state.selected;
  if (!sel || !LISTS[sel.list].map) { notice('Seleziona prima una riga della tabella, poi clicca sulla mappa.'); return; }
  const board = currentBoard();
  const rect = canvas.getBoundingClientRect();
  const scale = canvas.width / rect.width;
  const px = (e.clientX - rect.left) * scale;
  const py = (e.clientY - rect.top) * scale;
  const y = Math.floor(py / mapView.cell);
  const x = Math.floor(px / mapView.cell);
  if (x < 0 || y < 0 || x >= board.width || y >= board.height) return;
  const row = state.draft[sel.list][sel.index];
  if (sel.list === 'links') {
    row.from_board = board.id; row.from_x = x; row.from_y = y;
  } else {
    row.board_id = board.id; row.x = x; row.y = y;
  }
  notice('Casella (' + x + ', ' + y + ') su ' + board.name + '. Ricordati di salvare.');
  renderPanel();
}

// --- caratteristiche e razze ---

const CHAR_LABELS = { speed: 'Velocità', health: 'Vita', vision: 'Vista', strength: 'Forza' };
const charLabel = (c) => CHAR_LABELS[c.key] || c.key;

const raceStart = (race, c) => (c.kind === 'base' ? race[c.key] : (race.traits_min || {})[c.key] || 0);
const raceBonus = (race, c) => (c.kind === 'base' ? race['bonus_' + c.key] : (race.traits_bonus || {})[c.key] || 0);

// I limiti di una caratteristica per una razza: quelli del mondo, con quello che la razza cambia.
function effectiveBounds(race, c) {
  const o = (race.bounds || {})[c.key] || {};
  return { min: o.min ?? c.min, max: o.max ?? c.max, custom: o.min != null || o.max != null };
}

const backButton = (label, onclick) => el('button', { class: 'ghost', onclick }, '← ' + label);

function numberInput(value, onchange, opts = {}) {
  const i = el('input', { type: 'number', value: value ?? '', placeholder: opts.placeholder ?? '', onchange: (e) => onchange(e.target.value === '' ? null : Number(e.target.value)) });
  return i;
}

// -- caratteristiche: elenco

function renderCharList() {
  const rows = state.def.characteristics.map((c) => {
    const overriding = state.def.races.filter((r) => (r.bounds || {})[c.key]).length;
    return el('tr', {},
      el('td', {}, el('strong', {}, charLabel(c)), CHAR_LABELS[c.key] ? el('span', { class: 'muted' }, '  (' + c.key + ')') : ''),
      el('td', {}, c.kind === 'base' ? 'di base' : 'estesa'),
      el('td', { class: 'num' }, c.min), el('td', { class: 'num' }, c.max),
      el('td', {}, overriding ? overriding + (overriding === 1 ? ' razza' : ' razze') : '—'),
      el('td', {}, el('button', { class: 'ghost', onclick: () => { state.view = { type: 'char', key: c.key, draft: clone(c) }; renderPanel(); } }, 'Apri')));
  });
  return [
    el('h2', {}, 'Caratteristiche'),
    el('p', { class: 'muted' }, 'I valori che ha ogni pedina. Le quattro di base (velocità, vita, vista, forza) ci sono sempre; quelle estese (soldi, alcol...) le definisce il mondo. Ognuna ha un minimo e un massimo predefiniti, che ogni razza può cambiare: i valori iniziali e gli effetti degli oggetti restano dentro questi limiti.'),
    el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['Caratteristica', 'Tipo', 'Minimo', 'Massimo', 'Limiti propri di', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, rows))),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { state.view = { type: 'char', key: '', draft: { key: '', kind: 'extended', min: 0, max: 100 } }; renderPanel(); } }, '+ Nuova caratteristica')),
  ];
}

// -- caratteristiche: dettaglio

function renderCharDetail() {
  const v = state.view;
  const d = v.draft;
  const isNew = v.key === '';
  const keyInput = el('input', { type: 'text', value: d.key, maxlength: 20, placeholder: 'es. coraggio', disabled: !isNew, onchange: (e) => { d.key = e.target.value.trim(); } });
  const raceRows = state.def.races.map((r) => {
    if (isNew) return null;
    const b = effectiveBounds(r, d);
    return el('tr', {},
      el('td', {}, r.name), el('td', { class: 'num' }, b.min), el('td', { class: 'num' }, b.max),
      el('td', {}, b.custom ? el('span', { class: 'warn' }, 'personalizzato') : el('span', { class: 'muted' }, 'come il mondo')),
      el('td', {}, el('button', { class: 'ghost', onclick: () => { state.tab = 'races'; state.view = { type: 'race', id: r.id, draft: clone(r) }; renderTabs(); renderPanel(); } }, 'Apri razza')));
  }).filter(Boolean);

  const actions = [
    el('button', { onclick: () => saveChar(isNew) }, isNew ? 'Aggiungi' : 'Salva'),
  ];
  if (!isNew && d.kind === 'extended') {
    actions.push(el('button', { class: 'danger', onclick: () => deleteChar(d) }, 'Elimina'));
  }
  return [
    el('div', { class: 'crumb' }, backButton('Caratteristiche', () => { state.view = null; notice(''); renderPanel(); }),
      el('h2', {}, isNew ? 'Nuova caratteristica' : charLabel(d))),
    el('div', { class: 'card', style: 'max-width:760px;display:flex;flex-direction:column;gap:14px' },
      el('div', { class: 'grid2' },
        el('label', {}, 'Nome (minuscolo, senza spazi)', keyInput),
        el('label', {}, 'Tipo', el('input', { type: 'text', value: d.kind === 'base' ? 'di base (sempre presente)' : 'estesa', disabled: true }))),
      el('div', { class: 'grid2' },
        el('label', {}, 'Minimo predefinito', numberInput(d.min, (n) => { d.min = n ?? 0; })),
        el('label', {}, 'Massimo predefinito', numberInput(d.max, (n) => { d.max = n ?? 0; }))),
      el('p', { class: 'muted' }, 'Ogni razza può avere un minimo e un massimo suoi: si cambiano nella scheda della razza. Rinominare una caratteristica estesa vuol dire crearne una nuova ed eliminare la vecchia.'),
      el('div', { class: 'actions' }, ...actions)),
    isNew ? null : el('h3', {}, 'Limiti per razza'),
    isNew ? null : el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['Razza', 'Minimo', 'Massimo', '', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, raceRows))),
  ];
}

async function saveChar(isNew) {
  const d = state.view.draft;
  if (isNew && !/^[a-z][a-z0-9_]{1,19}$/.test(d.key)) {
    notice('Il nome deve avere da 2 a 20 caratteri: lettere minuscole, cifre e _, iniziando con una lettera.', 'bad');
    return;
  }
  const items = clone(state.def.characteristics);
  if (isNew) items.push({ key: d.key, kind: 'extended', min: d.min, max: d.max });
  else items.splice(items.findIndex((c) => c.key === state.view.key), 1, { key: d.key, kind: d.kind, min: d.min, max: d.max });
  if (await save('/characteristics', { items })) {
    state.view = isNew ? null : { type: 'char', key: d.key, draft: clone(state.def.characteristics.find((c) => c.key === d.key)) };
    renderPanel();
  }
}

async function deleteChar(d) {
  if (!confirm('Eliminare la caratteristica "' + d.key + '"? Viene tolta anche dalle razze (i valori già presi dalle pedine restano, ma non si vedono più).')) return;
  const items = clone(state.def.characteristics).filter((c) => c.key !== d.key);
  if (await save('/characteristics', { items })) { state.view = null; renderPanel(); }
}

// -- razze: elenco

function renderRaceList() {
  const range = (min, bonus) => (bonus > 0 ? min + '–' + (min + bonus) : String(min));
  const rows = state.def.races.map((r) => {
    const custom = Object.keys(r.bounds || {}).length;
    return el('tr', {},
      el('td', {}, el('strong', {}, r.name)),
      el('td', { class: 'muted' }, (r.description || '').length > 70 ? r.description.slice(0, 70) + '…' : r.description),
      el('td', { class: 'num' }, range(r.speed, r.bonus_speed)), el('td', { class: 'num' }, range(r.health, r.bonus_health)),
      el('td', { class: 'num' }, range(r.vision, r.bonus_vision)), el('td', { class: 'num' }, range(r.strength, r.bonus_strength)),
      el('td', {}, custom ? custom + (custom === 1 ? ' limite' : ' limiti') : '—'),
      el('td', {}, el('button', { class: 'ghost', onclick: () => { state.view = { type: 'race', id: r.id, draft: clone(r) }; renderPanel(); } }, 'Apri')));
  });
  return [
    el('h2', {}, 'Razze (' + state.def.races.length + ')'),
    el('p', { class: 'muted' }, 'Una razza dà alle sue pedine i valori iniziali (un minimo più un bonus casuale) e può cambiare i limiti delle caratteristiche. Nella tabella: velocità, vita, vista e forza iniziali (minimo o intervallo). Apri una razza per vederla e modificarla.'),
    el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['Razza', 'Descrizione', 'Velocità', 'Vita', 'Vista', 'Forza', 'Limiti propri', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, rows))),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { state.view = { type: 'race', id: '', draft: newRace() }; renderPanel(); } }, '+ Nuova razza')),
  ];
}

function newRace() {
  const r = { id: '', name: 'Nuova razza', description: '', look: '', bounds: {}, traits_min: {}, traits_bonus: {} };
  for (const c of state.def.characteristics) {
    if (c.kind === 'base') { r[c.key] = c.key === 'health' ? 100 : c.key === 'strength' ? 15 : c.key === 'vision' ? 3 : 2; r['bonus_' + c.key] = 0; }
  }
  return r;
}

// -- razze: dettaglio

function renderRaceDetail() {
  const v = state.view;
  const r = v.draft;
  r.bounds = r.bounds || {};
  r.traits_min = r.traits_min || {};
  r.traits_bonus = r.traits_bonus || {};
  const isNew = v.id === '';

  const name = el('input', { type: 'text', value: r.name, maxlength: 30, onchange: (e) => { r.name = e.target.value; } });
  const description = el('textarea', { rows: 3, maxlength: 300, onchange: (e) => { r.description = e.target.value; } });
  description.value = r.description || '';
  // Tutte le razze usano la stessa pedina standard: cambia il colore (il ruolo aggiunge il resto, per esempio la corona del campione).
  const look = el('select', { onchange: (e) => { r.look = e.target.value; } },
    el('option', { value: '' }, '(predefinito: salmone)'), ...state.def.looks.map((l) => el('option', { value: l }, l)));
  look.value = r.look || '';

  const setStart = (c, n) => {
    if (c.kind === 'base') r[c.key] = n ?? 0;
    else if (n) r.traits_min[c.key] = n; else delete r.traits_min[c.key];
  };
  const setBonus = (c, n) => {
    if (c.kind === 'base') r['bonus_' + c.key] = n ?? 0;
    else if (n) r.traits_bonus[c.key] = n; else delete r.traits_bonus[c.key];
  };
  const setLimit = (c, which, n) => {
    const o = r.bounds[c.key] || {};
    if (n === null) delete o[which]; else o[which] = n;
    if (o.min == null && o.max == null) delete r.bounds[c.key]; else r.bounds[c.key] = o;
    renderPanel();
  };

  const rows = state.def.characteristics.map((c) => {
    const eff = effectiveBounds(r, c);
    const start = raceStart(r, c);
    const outside = start < eff.min || start > eff.max;
    const o = r.bounds[c.key] || {};
    return el('tr', {},
      el('td', {}, el('strong', {}, charLabel(c)), el('div', { class: 'muted' }, c.kind === 'base' ? 'di base' : 'estesa')),
      el('td', {}, numberInput(start, (n) => { setStart(c, n); renderPanel(); })),
      el('td', {}, numberInput(raceBonus(r, c), (n) => { setBonus(c, n); renderPanel(); })),
      el('td', { class: o.min == null ? 'inherit' : '' }, numberInput(o.min, (n) => setLimit(c, 'min', n), { placeholder: c.min })),
      el('td', { class: o.max == null ? 'inherit' : '' }, numberInput(o.max, (n) => setLimit(c, 'max', n), { placeholder: c.max })),
      el('td', {}, outside ? el('span', { class: 'warn' }, '⚠ il valore iniziale esce dai limiti (' + eff.min + '–' + eff.max + '): sarà riportato dentro') : el('span', { class: 'muted' }, 'limiti ' + eff.min + '–' + eff.max)));
  });

  const actions = [el('button', { onclick: () => saveRace(isNew) }, isNew ? 'Crea razza' : 'Salva razza')];
  if (!isNew) actions.push(el('button', { class: 'danger', onclick: () => deleteRace(r) }, 'Elimina razza'));
  return [
    el('div', { class: 'crumb' }, backButton('Razze', () => { state.view = null; notice(''); renderPanel(); }), el('h2', {}, isNew ? 'Nuova razza' : r.name)),
    el('div', { class: 'card', style: 'display:flex;flex-direction:column;gap:14px' },
      el('div', { class: 'grid2' }, el('label', {}, 'Nome', name), el('label', {}, 'Descrizione', description),
        el('label', {}, 'Aspetto (colore della pedina)', look)),
      el('h3', { style: 'margin:0' }, 'Caratteristiche'),
      el('p', { class: 'muted', style: 'margin:0' }, 'Una pedina nasce con il valore iniziale più un numero a caso da 0 al bonus. I limiti lasciati vuoti (bordo tratteggiato, valore in grigio) sono quelli del mondo: scrivi un numero per cambiarli solo per questa razza.'),
      el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
        el('thead', {}, el('tr', {}, ...['Caratteristica', 'Valore iniziale', 'Bonus casuale', 'Limite minimo', 'Limite massimo', ''].map((h) => el('th', {}, h)))),
        el('tbody', {}, rows))),
      el('div', { class: 'actions' }, ...actions)),
  ];
}

async function saveRace(isNew) {
  const r = clone(state.view.draft);
  const items = clone(state.def.races);
  if (isNew) items.push(r);
  else items.splice(items.findIndex((x) => x.id === state.view.id), 1, r);
  if (await save('/races', { items })) {
    state.view = isNew ? null : { type: 'race', id: r.id, draft: clone(state.def.races.find((x) => x.id === r.id)) };
    renderPanel();
  }
}

async function deleteRace(r) {
  if (!confirm('Eliminare la razza "' + r.name + '"? Non si può se ci sono pedine o giocatori di questa razza.')) return;
  const items = clone(state.def.races).filter((x) => x.id !== r.id);
  if (await save('/races', { items })) { state.view = null; renderPanel(); }
}

// --- terrain tab: paint the cells of a board ---

const TERRAIN_COLORS = { '.': '#7bc96f', ',': '#e8d44d', '=': '#c98f5a', ':': '#a9b0b8', T: '#2e7d32', B: '#5aa85a', F: '#8b5a2b', '#': '#5b6270' };
const paint = { boardId: null, glyph: 'T', cell: 18, drawing: false };

function terrainRows(boardId) {
  if (!state.draft.terrain) state.draft.terrain = clone(state.def.terrain);
  return state.draft.terrain.find((t) => t.board_id === boardId).rows;
}

function renderTerrain() {
  if (!paint.boardId || !state.def.boards.some((b) => b.id === paint.boardId)) paint.boardId = state.def.boards[0].id;
  const canvas = el('canvas', { id: 'terrainMap' });
  const stop = () => { paint.drawing = false; };
  canvas.addEventListener('mousedown', (e) => { paint.drawing = true; paintAt(e, canvas); });
  canvas.addEventListener('mousemove', (e) => { if (paint.drawing) paintAt(e, canvas); });
  canvas.addEventListener('mouseup', stop);
  canvas.addEventListener('mouseleave', stop);
  const palette = state.def.terrain_kinds.map((k) => {
    const swatch = el('span', { class: 'dot' });
    swatch.style.background = TERRAIN_COLORS[k.glyph];
    return el('button', { class: k.glyph === paint.glyph ? '' : 'ghost', onclick: () => { paint.glyph = k.glyph; renderPanel(); } },
      swatch, k.name + (k.blocks ? ' (blocca)' : ''));
  });
  return [
    el('h2', {}, 'Terreno'),
    el('p', { class: 'muted' }, 'Ogni casella è erba se non scegli altro. Alberi, cespugli, staccionate e muri bloccano: nessuno ci può stare, quindi non si possono mettere sotto pedine, oggetti, strutture o passaggi. Scegli un tipo e dipingi trascinando sulla mappa; i puntini mostrano NPC (giallo), oggetti (azzurro) e passaggi (viola). Le altre board si salvano insieme.'),
    el('label', { style: 'max-width:240px' }, 'Board',
      el('select', { onchange: (e) => { paint.boardId = e.target.value; renderPanel(); } },
        ...state.def.boards.map((b) => el('option', { value: b.id, selected: b.id === paint.boardId }, b.name)))),
    el('div', { class: 'actions' }, ...palette),
    el('div', { class: 'mapbox' }, canvas),
    el('div', { class: 'actions' },
      el('button', { onclick: () => save('/terrain', { items: state.draft.terrain || state.def.terrain }) }, 'Salva terreno'),
      el('button', { class: 'ghost', onclick: () => { delete state.draft.terrain; renderPanel(); notice(''); } }, 'Annulla modifiche')),
  ];
}

function drawTerrain() {
  const canvas = $('terrainMap');
  if (!canvas) return;
  const board = state.def.boards.find((b) => b.id === paint.boardId);
  const rows = terrainRows(board.id);
  const c = paint.cell;
  canvas.width = board.width * c + 1;
  canvas.height = board.height * c + 1;
  const ctx = canvas.getContext('2d');
  rows.forEach((row, y) => [...row].forEach((glyph, x) => {
    ctx.fillStyle = TERRAIN_COLORS[glyph] || '#f0f';
    ctx.fillRect(x * c, y * c, c - 1, c - 1);
  }));
  const dot = (x, y, color) => { ctx.fillStyle = color; ctx.beginPath(); ctx.arc(x * c + c / 2, y * c + c / 2, c / 4, 0, Math.PI * 2); ctx.fill(); ctx.strokeStyle = '#000'; ctx.stroke(); };
  state.def.npcs.filter((n) => n.board_id === board.id).forEach((n) => dot(n.x, n.y, '#ffd84a'));
  state.def.items.filter((n) => n.board_id === board.id).forEach((n) => dot(n.x, n.y, '#6fb3ff'));
  state.def.links.forEach((l) => {
    if (l.from_board === board.id) dot(l.from_x, l.from_y, '#c084fc');
    if (l.to_board === board.id) dot(l.to_x, l.to_y, '#c084fc');
  });
}

function paintAt(e, canvas) {
  const board = state.def.boards.find((b) => b.id === paint.boardId);
  const rect = canvas.getBoundingClientRect();
  const scale = canvas.width / rect.width;
  const x = Math.floor((e.clientX - rect.left) * scale / paint.cell);
  const y = Math.floor((e.clientY - rect.top) * scale / paint.cell);
  if (x < 0 || y < 0 || x >= board.width || y >= board.height) return;
  const rows = terrainRows(board.id);
  if (rows[y][x] === paint.glyph) return;
  rows[y] = rows[y].slice(0, x) + paint.glyph + rows[y].slice(x + 1);
  drawTerrain();
}

// --- players tab ---

function renderPlayers() {
  const goals = state.def.goals.filter((g) => g.scope === 'individual');
  const goalTitle = (id) => (state.def.goals.find((g) => g.id === id) || {}).title || '—';
  const manual = state.def.effective_rules.goal_assignment === 'manual';
  const rows = state.def.players.map((p) => {
    const select = el('select', {}, ...goals.map((g) => el('option', { value: g.id }, g.title)));
    return el('tr', {},
      el('td', {}, p.username), el('td', {}, p.race || '—'), el('td', {}, p.points), el('td', {}, p.units),
      el('td', {}, p.kills + ' / ' + p.champion_kills), el('td', {}, p.pickups), el('td', {}, p.talks),
      el('td', {}, goalTitle(p.goal_id) + (p.completed_goals ? ' (' + p.completed_goals + ' completati)' : '')),
      el('td', {}, goals.length === 0 ? '' : el('div', { class: 'actions', style: 'margin:0' }, select,
        el('button', { class: 'ghost', onclick: async () => {
          try {
            await api('POST', '/admin/api/worlds/' + state.worldId + '/assign', { player_id: p.id, goal_id: select.value });
            notice('Obiettivo assegnato a ' + p.username + '.', 'ok');
            state.def = await api('GET', '/admin/api/worlds/' + state.worldId);
            renderPanel();
          } catch (err) { notice(err.message, 'bad'); }
        } }, 'Assegna'))));
  });
  return [
    el('h2', {}, 'Giocatori (' + rows.length + ')'),
    el('p', { class: 'muted' }, manual
      ? 'Gli obiettivi individuali si assegnano a mano (goal_assignment = manual): scegline uno e premi Assegna. Sostituisce quello in corso.'
      : 'Gli obiettivi individuali sono assegnati a caso dal sistema (goal_assignment = random); puoi comunque sostituirne uno a mano.'),
    el('div', { class: 'tablewrap' }, el('table', {},
      el('thead', {}, el('tr', {}, ...['Giocatore', 'Razza', 'Punti', 'Pedine', 'Sconfitte / campioni', 'Oggetti', 'NPC', 'Obiettivo', 'Assegna'].map((h) => el('th', {}, h)))),
      el('tbody', {}, rows))),
    el('div', { class: 'actions' }, el('button', { class: 'ghost', onclick: async () => { state.def = await api('GET', '/admin/api/worlds/' + state.worldId); renderPanel(); } }, 'Aggiorna')),
  ];
}

// --- start-up ---

$('doLogin').addEventListener('click', doLogin);
$('sideToggle').addEventListener('click', () => setSideCollapsed(!$('side').classList.contains('collapsed')));
try { setSideCollapsed(localStorage.getItem(SIDE_KEY) === '1' || window.innerWidth < 700); } catch (_) { setSideCollapsed(window.innerWidth < 700); }
$('password').addEventListener('keydown', (e) => { if (e.key === 'Enter') doLogin(); });
$('logout').addEventListener('click', logout);
$('worldPicker').addEventListener('change', async (e) => { state.worldId = e.target.value; state.tab = 'world'; notice(''); await loadWorld(); });
$('newWorld').addEventListener('click', async () => {
  const name = prompt('Nome del nuovo mondo');
  if (!name) return;
  try {
    const out = await api('POST', '/worlds', { name, description: '' });
    await start();
    state.worldId = out.id;
    $('worldPicker').value = out.id;
    await loadWorld();
  } catch (e) { alert(e.message); }
});

(async function init() {
  let token = null, user = '';
  try { token = sessionStorage.getItem(TOKEN_KEY); user = sessionStorage.getItem(USER_KEY) || ''; } catch (_) { /* ignore */ }
  if (!token) { show('login'); return; }
  state.token = token;
  state.username = user;
  try {
    await start();
  } catch (e) {
    logout();
  }
})();
