// Admin web app: static page, no build step. It talks to the backend it is served from
// (/auth/login, /worlds, /admin/api/...). Everything shown comes from the server; nothing is
// decided here except how to lay it out. All DOM is built with textContent (never innerHTML with
// data), so names typed by players cannot inject markup.
'use strict';

const $ = (id) => document.getElementById(id);
const TOKEN_KEY = 'thegame.admin.token';
const WORLD_KEY = 'thegame.admin.world';
const USER_KEY = 'thegame.admin.user';

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
  ['world', 'Mondo'],
  ['rules', 'Regole'],
  ['boards', 'Board'],
  ['links', 'Passaggi'],
  ['races', 'Razze'],
  ['compat', 'Compatibilità'],
  ['goals', 'Obiettivi'],
  ['npcs', 'NPC'],
  ['items', 'Oggetti'],
  ['players', 'Giocatori'],
];

function renderTabs() {
  $('tabs').replaceChildren(...TABS.map(([id, label]) =>
    el('button', { class: id === state.tab ? 'active' : '', onclick: () => { state.tab = id; state.selected = null; notice(''); renderTabs(); renderPanel(); } }, label)));
}

function renderPanel() {
  const panel = $('panel');
  const renderer = { world: renderWorld, rules: renderRules, players: renderPlayers }[state.tab];
  panel.replaceChildren(...(renderer ? renderer() : [renderList(state.tab)]));
  drawMap(); // no-op unless this tab has a map
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
    el('p', { class: 'muted' }, 'Le caselle con il bordo giallo differiscono dai valori predefiniti. Si salvano solo le differenze: se un giorno cambia un valore predefinito, i mondi che non lo hanno modificato lo seguono.'),
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
    help: 'Ogni board è una zona del mondo. Le griglie sono "square" (quadrata) o "hex" (esagonale). Una board non si può cancellare se ci sono pedine o strutture dei giocatori; se la rimpicciolisci, niente deve restare fuori.',
    cols: [
      { key: 'name', label: 'Nome', kind: 'text' },
      { key: 'width', label: 'Larghezza', kind: 'number' },
      { key: 'height', label: 'Altezza', kind: 'number' },
      { key: 'grid', label: 'Griglia', kind: 'select', options: () => [['square', 'square'], ['hex', 'hex']] },
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
  races: {
    title: 'Razze', path: '/races', blank: () => ({ id: '', name: 'Nuova razza', description: '', speed: 2, health: 100, vision: 3, strength: 15, bonus_speed: 0, bonus_health: 0, bonus_vision: 0, bonus_strength: 0, traits_min: {}, traits_bonus: {} }),
    help: 'Una pedina di questa razza nasce con almeno i valori "min" più un numero a caso da 0 al "bonus". Le caratteristiche estese si scrivono nome=valore (i nomi validi sono quelli di trait_names nelle regole). Una razza usata da pedine o giocatori non si può cancellare.',
    cols: [
      { key: 'name', label: 'Nome', kind: 'text' },
      { key: 'description', label: 'Descrizione', kind: 'area' },
      { key: 'speed', label: 'Velocità', kind: 'number' }, { key: 'bonus_speed', label: '+ casuale', kind: 'number' },
      { key: 'health', label: 'Vita', kind: 'number' }, { key: 'bonus_health', label: '+ casuale', kind: 'number' },
      { key: 'vision', label: 'Vista', kind: 'number' }, { key: 'bonus_vision', label: '+ casuale', kind: 'number' },
      { key: 'strength', label: 'Forza', kind: 'number' }, { key: 'bonus_strength', label: '+ casuale', kind: 'number' },
      { key: 'traits_min', label: 'Caratteristiche min', kind: 'traits' },
      { key: 'traits_bonus', label: 'Caratteristiche +casuale', kind: 'traits' },
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
    blank: () => ({ id: '', board_id: state.def.boards[0].id, name: 'Nuovo NPC', description: '', x: 0, y: 0, speed: 0, health: 100, vision: 3, strength: 10, dialogue: '', race_id: '', traits: {} }),
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

// Cell centre on the canvas. Square grids are plain rows; hex grids are "odd-r" (odd rows shifted).
function cellCenter(board, x, y) {
  const c = mapView.cell;
  const shift = board.grid === 'hex' && y % 2 === 1 ? c / 2 : 0;
  return [x * c + c / 2 + shift, y * c + c / 2];
}

function drawMap() {
  const canvas = $('map');
  if (!canvas) return;
  const board = currentBoard();
  const select = $('mapBoard');
  if (select) select.value = board.id;
  const c = mapView.cell;
  canvas.width = board.width * c + (board.grid === 'hex' ? c / 2 : 0) + 1;
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
  const x = Math.floor((px - (board.grid === 'hex' && y % 2 === 1 ? mapView.cell / 2 : 0)) / mapView.cell);
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
