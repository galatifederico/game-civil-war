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
  ['classes', 'Classi', '▲'],
  ['races', 'Razze', '☺'],
  ['compat', 'Compatibilità', '⚭'],
  ['board', 'Board', '▦'],
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
    world: renderWorld, rules: renderRules, players: renderPlayers, board: renderBoardPage,
    races: () => (state.view && state.view.type === 'race' ? renderRaceDetail() : renderRaceList()),
    classes: () => (state.view && state.view.type === 'class' ? renderClassDetail() : renderClassList()),
    characteristics: () => (state.view && state.view.type === 'char' ? renderCharDetail() : renderCharList()),
    npcs: () => (state.view && state.view.type === 'npc' ? renderNpcDetail() : renderNpcList()),
    items: () => (state.view && state.view.type === 'item' ? renderItemDetail() : renderItemList()),
  }[state.tab];
  // replaceChildren stringifies anything that is not a Node (so a bare null would show up as the
  // text "null"): renderers may return one for a part that does not apply right now, drop those.
  panel.replaceChildren(...(renderer ? renderer() : [renderList(state.tab)]).filter(Boolean));
  drawBoardMap(); // no-op unless the Board page (with its map) is showing
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
};

function cellInput(row, col) {
  const value = getPath(row, col.key);
  const set = (v) => setPath(row, col.key, v);
  switch (col.kind) {
    case 'number':
      return el('input', { type: 'number', value: value ?? 0, onchange: (e) => set(Number(e.target.value)) });
    case 'select': {
      const select = el('select', { onchange: (e) => set(e.target.value) },
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
    ...list.cols.map((c) => el('td', {}, cellInput(row, c))),
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
  return el('div', {}, ...parts);
}

function selectRow(list, index, rerender) {
  state.selected = { list, index };
  if (rerender) { renderPanel(); return; }
  // Mark the row without rebuilding the table (that would drop the focus).
  document.querySelectorAll('#panel tbody tr').forEach((tr, i) => tr.classList.toggle('selected', i === index));
}

function legend(color, text) {
  const dot = el('span', { class: 'dot' });
  dot.style.background = color;
  return el('span', {}, dot, text);
}

// --- board page: one map for terrain, passages, NPCs and items -----------------------------
//
// board+terrain+links used to be three separate tabs; NPCs and items had their own small map
// each. They are now one page: a mode picks what a click on the map does, everything else shows
// as dots so you always see where things are. Terreno paints by dragging; the other modes place
// whatever row is "armed" (boardEditor.selected) on the clicked cell.

const TERRAIN_COLORS = { '.': '#7bc96f', ',': '#e8d44d', '=': '#c98f5a', ':': '#a9b0b8', T: '#2e7d32', B: '#5aa85a', F: '#8b5a2b', '#': '#5b6270' };
const MODES = [['terrain', 'Terreno'], ['links', 'Passaggi'], ['npcs', 'NPC'], ['items', 'Oggetti']];
const boardEditor = { boardId: null, cell: 20, mode: 'terrain', glyph: 'T', drawing: false, selected: null };

function terrainRows(boardId) {
  if (!state.draft.terrain) state.draft.terrain = clone(state.def.terrain);
  return state.draft.terrain.find((t) => t.board_id === boardId).rows;
}

function currentBoardEditor() {
  if (!boardEditor.boardId || !state.def.boards.some((b) => b.id === boardEditor.boardId)) boardEditor.boardId = state.def.boards[0].id;
  return state.def.boards.find((b) => b.id === boardEditor.boardId);
}

function armPlacement(list, index) {
  boardEditor.mode = list;
  boardEditor.selected = { list, index };
  renderPanel();
}

function renderBoardPage() {
  const board = currentBoardEditor();
  const canvas = el('canvas', { id: 'boardMap' });
  const stop = () => { boardEditor.drawing = false; };
  canvas.addEventListener('mousedown', (e) => { boardEditor.drawing = true; clickBoardMap(e, canvas); });
  canvas.addEventListener('mousemove', (e) => { if (boardEditor.drawing && boardEditor.mode === 'terrain') paintBoardMap(e, canvas); });
  canvas.addEventListener('mouseup', stop);
  canvas.addEventListener('mouseleave', stop);

  const modeButtons = MODES.map(([m, label]) => el('button', {
    class: boardEditor.mode === m ? '' : 'ghost',
    onclick: () => { boardEditor.mode = m; boardEditor.selected = null; renderPanel(); notice(''); },
  }, label));

  const parts = [
    el('h2', {}, 'Board'),
    el('p', { class: 'muted' }, 'Le aree del mondo: dimensioni e griglia qui sotto; terreno, passaggi, NPC e oggetti si vedono e si spostano tutti nella stessa mappa, più sotto.'),
    renderList('boards'),
    el('h2', {}, 'Mappa'),
    el('p', { class: 'muted' }, 'Scegli cosa modificare, poi clicca sulla mappa (trascina, per il terreno). Le altre cose restano visibili come puntini colorati.'),
    el('label', { style: 'max-width:280px' }, 'Board mostrata',
      el('select', { onchange: (e) => { boardEditor.boardId = e.target.value; boardEditor.selected = null; renderPanel(); } },
        ...state.def.boards.map((b) => el('option', { value: b.id, selected: b.id === board.id }, b.name)))),
    el('div', { class: 'actions' }, ...modeButtons),
    boardEditor.mode === 'terrain' ? renderTerrainPalette() : null,
    el('div', { class: 'legend' },
      legend('#ffd84a', 'NPC'), legend('#6fb3ff', 'oggetto'), legend('#c084fc', 'passaggio'), legend('#ffffff', 'selezionato')),
    el('div', { class: 'mapbox' }, canvas),
  ];
  if (boardEditor.mode === 'terrain') {
    parts.push(el('div', { class: 'actions' },
      el('button', { onclick: () => save('/terrain', { items: state.draft.terrain || state.def.terrain }) }, 'Salva terreno'),
      el('button', { class: 'ghost', onclick: () => { delete state.draft.terrain; renderPanel(); notice(''); } }, 'Annulla modifiche')));
  } else if (boardEditor.mode === 'links') {
    parts.push(renderBoardLinks());
  } else {
    parts.push(renderBoardEntities(boardEditor.mode));
  }
  return parts;
}

function renderTerrainPalette() {
  return el('div', { class: 'actions' }, ...state.def.terrain_kinds.map((k) => {
    const swatch = el('span', { class: 'dot' });
    swatch.style.background = TERRAIN_COLORS[k.glyph];
    return el('button', { class: k.glyph === boardEditor.glyph ? '' : 'ghost', onclick: () => { boardEditor.glyph = k.glyph; renderPanel(); } },
      swatch, k.name + (k.blocks ? ' (blocca)' : ''));
  }));
}

// The passages table: the same fields as before, plus a "Posiziona" button per row that arms it
// for the map (a click there sets the end that is on the board currently shown).
function renderBoardLinks() {
  if (!state.draft.links) state.draft.links = clone(state.def.links);
  const rows = state.draft.links;
  const cols = LISTS.links.cols;
  const table = el('table', { class: 'list' },
    el('thead', {}, el('tr', {}, ...cols.map((c) => el('th', {}, c.label)), el('th', {}), el('th', {}))),
    el('tbody', {}, ...rows.map((row, i) => el('tr',
      { class: boardEditor.selected && boardEditor.selected.list === 'links' && boardEditor.selected.index === i ? 'selected' : '' },
      ...cols.map((c) => el('td', {}, cellInput(row, c))),
      el('td', {}, el('button', { class: 'ghost', onclick: () => armPlacement('links', i) }, 'Posiziona')),
      el('td', {}, el('button', { class: 'danger', title: 'Elimina', onclick: () => { rows.splice(i, 1); boardEditor.selected = null; renderPanel(); } }, '✕'))))));
  table.addEventListener('change', () => drawBoardMap());
  return el('div', {},
    el('h3', {}, 'Passaggi (' + rows.length + ')'),
    el('p', { class: 'muted' }, LISTS.links.help + ' "Posiziona" e poi un clic sulla mappa muove l\'estremo che sta sulla board mostrata.'),
    el('div', { class: 'tablewrap' }, table),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { rows.push(LISTS.links.blank()); armPlacement('links', rows.length - 1); } }, '+ Aggiungi'),
      el('button', { onclick: () => save('/links', { items: rows }) }, 'Salva passaggi'),
      el('button', { class: 'ghost', onclick: () => { delete state.draft.links; boardEditor.selected = null; renderPanel(); notice(''); } }, 'Annulla modifiche')));
}

// The NPC/item quick-position list for the map: name, where they are, "Posiziona" to arm them for
// a click on the map, and "Apri scheda" to their full page (item 3) for everything else.
function renderBoardEntities(name) {
  if (!state.draft[name]) state.draft[name] = clone(state.def[name]);
  const rows = state.draft[name];
  const label = name === 'npcs' ? 'NPC' : 'Oggetti';
  const body = rows.map((row, i) => el('tr',
    { class: boardEditor.selected && boardEditor.selected.list === name && boardEditor.selected.index === i ? 'selected' : '' },
    el('td', {}, spritePreview(name === 'npcs' ? (row.sprite || 'wanderer') : null, 26)),
    el('td', {}, row.name),
    el('td', {}, (state.def.boards.find((b) => b.id === row.board_id) || {}).name || '?'),
    el('td', { class: 'num' }, row.x + ', ' + row.y),
    el('td', {}, el('button', { class: 'ghost', onclick: () => armPlacement(name, i) }, 'Posiziona')),
    el('td', {}, el('button', { class: 'ghost', onclick: () => {
      state.tab = name; state.view = { type: name === 'npcs' ? 'npc' : 'item', id: row.id, draft: clone(row) }; renderTabs(); renderPanel();
    } }, 'Apri scheda'))));
  return el('div', {},
    el('h3', {}, label + ' (' + rows.length + ')'),
    el('p', { class: 'muted' }, '"Posiziona" e poi un clic sulla mappa sposta la riga. Il resto (nome, statistiche, sprite...) si modifica nella scheda.'),
    el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['', 'Nome', 'Board', 'Casella', '', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, body))),
    el('div', { class: 'actions' },
      el('button', { onclick: () => save('/' + name, { items: rows }) }, 'Salva posizioni ' + label.toLowerCase()),
      el('button', { class: 'ghost', onclick: () => { delete state.draft[name]; boardEditor.selected = null; renderPanel(); notice(''); } }, 'Annulla modifiche')));
}

function cellFromEvent(e, canvas, board) {
  const rect = canvas.getBoundingClientRect();
  const scale = canvas.width / rect.width;
  const x = Math.floor((e.clientX - rect.left) * scale / boardEditor.cell);
  const y = Math.floor((e.clientY - rect.top) * scale / boardEditor.cell);
  return x < 0 || y < 0 || x >= board.width || y >= board.height ? null : { x, y };
}

function paintBoardMap(e, canvas) {
  const board = currentBoardEditor();
  const cell = cellFromEvent(e, canvas, board);
  if (!cell) return;
  const rows = terrainRows(board.id);
  if (rows[cell.y][cell.x] === boardEditor.glyph) return;
  rows[cell.y] = rows[cell.y].slice(0, cell.x) + boardEditor.glyph + rows[cell.y].slice(cell.x + 1);
  drawBoardMap();
}

function clickBoardMap(e, canvas) {
  if (boardEditor.mode === 'terrain') { paintBoardMap(e, canvas); return; }
  const board = currentBoardEditor();
  const cell = cellFromEvent(e, canvas, board);
  if (!cell) return;
  const sel = boardEditor.selected;
  if (!sel || sel.list !== boardEditor.mode) { notice('Premi prima "Posiziona" sulla riga da spostare, poi clicca sulla mappa.', 'bad'); return; }
  if (boardEditor.mode === 'links') {
    const row = state.draft.links[sel.index];
    // A click moves whichever end is (or becomes) the one on the board shown.
    if (row.to_board === board.id && row.from_board !== board.id) { row.to_x = cell.x; row.to_y = cell.y; }
    else { row.from_board = board.id; row.from_x = cell.x; row.from_y = cell.y; }
  } else {
    const row = state.draft[boardEditor.mode][sel.index];
    row.board_id = board.id; row.x = cell.x; row.y = cell.y;
  }
  notice('Casella (' + cell.x + ', ' + cell.y + ') su ' + board.name + '. Ricordati di salvare.', 'ok');
  drawBoardMap();
}

function drawBoardMap() {
  const canvas = $('boardMap');
  if (!canvas) return;
  const board = currentBoardEditor();
  const rows = terrainRows(board.id);
  const c = boardEditor.cell;
  canvas.width = board.width * c + 1;
  canvas.height = board.height * c + 1;
  const ctx = canvas.getContext('2d');
  rows.forEach((row, y) => [...row].forEach((glyph, x) => {
    ctx.fillStyle = TERRAIN_COLORS[glyph] || '#f0f';
    ctx.fillRect(x * c, y * c, c - 1, c - 1);
  }));
  const dot = (x, y, color, ring) => {
    ctx.fillStyle = color;
    ctx.beginPath();
    ctx.arc(x * c + c / 2, y * c + c / 2, c / 3, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = ring ? '#fff' : '#000';
    ctx.lineWidth = ring ? 2 : 1;
    ctx.stroke();
    ctx.lineWidth = 1;
  };
  const isSel = (list, i) => boardEditor.selected && boardEditor.selected.list === list && boardEditor.selected.index === i;
  (state.draft.npcs || state.def.npcs).forEach((n, i) => { if (n.board_id === board.id) dot(n.x, n.y, '#ffd84a', isSel('npcs', i)); });
  (state.draft.items || state.def.items).forEach((n, i) => { if (n.board_id === board.id) dot(n.x, n.y, '#6fb3ff', isSel('items', i)); });
  (state.draft.links || state.def.links).forEach((l, i) => {
    if (l.from_board === board.id) dot(l.from_x, l.from_y, '#c084fc', isSel('links', i));
    if (l.to_board === board.id) dot(l.to_x, l.to_y, '#c084fc', isSel('links', i));
  });
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
      el('td', {}, spritePreview(raceSpriteName(r.look), 32)),
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
      el('thead', {}, el('tr', {}, ...['', 'Razza', 'Descrizione', 'Velocità', 'Vita', 'Vista', 'Forza', 'Limiti propri', ''].map((h) => el('th', {}, h)))),
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
  const look = el('select', { onchange: (e) => { r.look = e.target.value; renderPanel(); } },
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
      el('div', { style: 'display:flex;gap:16px;align-items:flex-end;flex-wrap:wrap' },
        spritePreview(raceSpriteName(r.look), 64),
        el('label', {}, 'Aspetto (colore della pedina)', look)),
      el('div', { class: 'grid2' }, el('label', {}, 'Nome', name), el('label', {}, 'Descrizione', description)),
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

// --- sprite preview: the same pixel-art pawn the game draws, cropped from art/characters.png
// (built by tools/sprites/build_characters.py from img/pawn.png). Used by NPC, Razze and Classi so
// you see the look you are choosing, not just its name (art/characters-layout.json says where each
// variant sits in the sheet: one row of frames, the first column is the pose facing the viewer).

const spriteAtlas = { img: null, layout: null, loading: null };

function ensureSpriteAtlas() {
  if (spriteAtlas.loading) return spriteAtlas.loading;
  spriteAtlas.loading = Promise.all([
    fetch('art/characters-layout.json').then((r) => (r.ok ? r.json() : null)).catch(() => null),
    new Promise((resolve) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => resolve(null);
      img.src = 'art/characters.png';
    }),
  ]).then(([layout, img]) => {
    spriteAtlas.layout = layout;
    spriteAtlas.img = img;
    renderPanel(); // the first previews were blank while this loaded; redraw now that it's here
  });
  return spriteAtlas.loading;
}

// The pedina standard in a given colour ("" or unknown -> the plain, uncoloured one): what a race
// or a class looks like when nothing more specific applies.
const raceSpriteName = (look) => (look ? 'minor-' + look : 'minor');

// A small canvas with one character's sprite; name can be null or unknown (stays blank).
function spritePreview(name, size) {
  const canvas = el('canvas', { width: size, height: size, class: 'sprite-preview' });
  canvas.style.width = canvas.style.height = size + 'px';
  if (name) { ensureSpriteAtlas(); drawSpritePreview(canvas, name); }
  return canvas;
}

function drawSpritePreview(canvas, name) {
  const { img, layout } = spriteAtlas;
  const ctx = canvas.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  if (!img || !layout) return; // still loading; ensureSpriteAtlas() redraws everything once it is
  const entry = layout.characters.find((c) => c.name === name);
  if (entry) ctx.drawImage(img, 0, entry.row * layout.cell, layout.cell, layout.cell, 0, 0, canvas.width, canvas.height);
}

// -- classi: elenco -------------------------------------------------------------------------
//
// Una classe è come una razza, ma la classe di una pedina può cambiare nel tempo (la razza no) e
// non ha valori iniziali propri: cambia solo i limiti delle caratteristiche. Quando una pedina ha
// sia una razza sia una classe, i limiti finali sono l'unione dei due: per il minimo e per il
// massimo, sempre il valore maggiore tra quello della razza e quello della classe (vedi
// game.World.BoundsFor). Il server non ha ancora un modo per assegnare una classe a una pedina:
// questa pagina la definisce soltanto.

function renderClassList() {
  const rows = state.def.classes.map((c) => {
    const custom = Object.keys(c.bounds || {}).length;
    return el('tr', {},
      el('td', {}, spritePreview(raceSpriteName(c.look), 32)),
      el('td', {}, el('strong', {}, c.name)),
      el('td', { class: 'muted' }, (c.description || '').length > 70 ? c.description.slice(0, 70) + '…' : c.description),
      el('td', {}, custom ? custom + (custom === 1 ? ' limite' : ' limiti') : '—'),
      el('td', {}, el('button', { class: 'ghost', onclick: () => { state.view = { type: 'class', id: c.id, draft: clone(c) }; renderPanel(); } }, 'Apri')));
  });
  return [
    el('h2', {}, 'Classi (' + state.def.classes.length + ')'),
    el('p', { class: 'muted' }, 'Una classe è come una razza, ma quella di una pedina può cambiare nel tempo e non dà valori iniziali: cambia solo i limiti delle caratteristiche (ed eventualmente il colore). Se una pedina ha sia razza sia classe, per ogni limite vince sempre il valore maggiore tra i due. Apri una classe per vederla e modificarla.'),
    el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['', 'Classe', 'Descrizione', 'Limiti propri', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, rows))),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { state.view = { type: 'class', id: '', draft: { id: '', name: 'Nuova classe', description: '', look: '', bounds: {} } }; renderPanel(); } }, '+ Nuova classe')),
  ];
}

// -- classi: dettaglio -----------------------------------------------------------------------

function renderClassDetail() {
  const v = state.view;
  const c = v.draft;
  c.bounds = c.bounds || {};
  const isNew = v.id === '';

  const name = el('input', { type: 'text', value: c.name, maxlength: 30, onchange: (e) => { c.name = e.target.value; } });
  const description = el('textarea', { rows: 3, maxlength: 300, onchange: (e) => { c.description = e.target.value; } });
  description.value = c.description || '';
  const look = el('select', { onchange: (e) => { c.look = e.target.value; renderPanel(); } },
    el('option', { value: '' }, '(usa il colore della razza)'), ...state.def.looks.map((l) => el('option', { value: l }, l)));
  look.value = c.look || '';

  const setLimit = (ch, which, n) => {
    const o = c.bounds[ch.key] || {};
    if (n === null) delete o[which]; else o[which] = n;
    if (o.min == null && o.max == null) delete c.bounds[ch.key]; else c.bounds[ch.key] = o;
    renderPanel();
  };
  const rows = state.def.characteristics.map((ch) => {
    const eff = effectiveBounds(c, ch);
    const o = c.bounds[ch.key] || {};
    return el('tr', {},
      el('td', {}, el('strong', {}, charLabel(ch)), el('div', { class: 'muted' }, ch.kind === 'base' ? 'di base' : 'estesa')),
      el('td', { class: o.min == null ? 'inherit' : '' }, numberInput(o.min, (n) => setLimit(ch, 'min', n), { placeholder: ch.min })),
      el('td', { class: o.max == null ? 'inherit' : '' }, numberInput(o.max, (n) => setLimit(ch, 'max', n), { placeholder: ch.max })),
      el('td', {}, o.min == null && o.max == null ? el('span', { class: 'muted' }, 'come il mondo (' + eff.min + '–' + eff.max + ')') : el('span', { class: 'warn' }, 'limiti propri ' + eff.min + '–' + eff.max)));
  });

  const actions = [el('button', { onclick: () => saveClass(isNew) }, isNew ? 'Crea classe' : 'Salva classe')];
  if (!isNew) actions.push(el('button', { class: 'danger', onclick: () => deleteClass(c) }, 'Elimina classe'));
  return [
    el('div', { class: 'crumb' }, backButton('Classi', () => { state.view = null; notice(''); renderPanel(); }), el('h2', {}, isNew ? 'Nuova classe' : c.name)),
    el('div', { class: 'card', style: 'display:flex;flex-direction:column;gap:14px' },
      el('div', { class: 'grid2' },
        el('div', { style: 'display:flex;gap:12px;align-items:flex-end' }, spritePreview(raceSpriteName(c.look), 64),
          el('label', { style: 'flex:1' }, 'Aspetto (vuoto = quello della razza)', look)),
        el('label', {}, 'Nome', name)),
      el('label', {}, 'Descrizione', description),
      el('h3', { style: 'margin:0' }, 'Limiti delle caratteristiche'),
      el('p', { class: 'muted', style: 'margin:0' }, 'Una classe non ha valori iniziali: solo limiti. Quelli lasciati vuoti (bordo tratteggiato) sono quelli del mondo. Se la pedina ha anche una razza, alla fine vale sempre il limite maggiore tra i due.'),
      el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
        el('thead', {}, el('tr', {}, ...['Caratteristica', 'Limite minimo', 'Limite massimo', ''].map((h) => el('th', {}, h)))),
        el('tbody', {}, rows))),
      el('div', { class: 'actions' }, ...actions)),
  ];
}

async function saveClass(isNew) {
  const c = clone(state.view.draft);
  const items = clone(state.def.classes);
  if (isNew) items.push(c);
  else items.splice(items.findIndex((x) => x.id === state.view.id), 1, c);
  if (await save('/classes', { items })) {
    state.view = isNew ? null : { type: 'class', id: c.id, draft: clone(state.def.classes.find((x) => x.id === c.id)) };
    renderPanel();
  }
}

async function deleteClass(c) {
  if (!confirm('Eliminare la classe "' + c.name + '"? Non si può se qualche pedina la indossa.')) return;
  const items = clone(state.def.classes).filter((x) => x.id !== c.id);
  if (await save('/classes', { items })) { state.view = null; renderPanel(); }
}

// -- NPC: elenco -----------------------------------------------------------------------------

function renderNpcList() {
  const rows = state.def.npcs.map((n) => el('tr', {},
    el('td', {}, spritePreview(n.sprite || 'wanderer', 32)),
    el('td', {}, el('strong', {}, n.name)),
    el('td', { class: 'muted' }, (n.description || '').length > 60 ? n.description.slice(0, 60) + '…' : n.description),
    el('td', {}, (state.def.boards.find((b) => b.id === n.board_id) || {}).name || '?'),
    el('td', { class: 'num' }, n.x + ', ' + n.y),
    el('td', {}, n.race_id ? (state.def.races.find((r) => r.id === n.race_id) || {}).name || '—' : '—'),
    el('td', {}, el('button', { class: 'ghost', onclick: () => { state.view = { type: 'npc', id: n.id, draft: clone(n) }; renderPanel(); } }, 'Apri'))));
  return [
    el('h2', {}, 'NPC (' + state.def.npcs.length + ')'),
    el('p', { class: 'muted' }, 'Le pedine che non controlla nessuno. Apri un NPC per vederlo e modificarlo, oppure vai alla pagina Board per spostarlo sulla mappa.'),
    el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['', 'Nome', 'Descrizione', 'Board', 'Casella', 'Razza', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, rows))),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { state.view = { type: 'npc', id: '', draft: newNpc() }; renderPanel(); } }, '+ Nuovo NPC')),
  ];
}

function newNpc() {
  return { id: '', board_id: state.def.boards[0].id, name: 'Nuovo NPC', description: '', x: 0, y: 0, speed: 0, health: 100, vision: 3, strength: 10, dialogue: '', race_id: '', traits: {}, sprite: '' };
}

// -- NPC: dettaglio ----------------------------------------------------------------------------

function renderNpcDetail() {
  const v = state.view;
  const n = v.draft;
  n.traits = n.traits || {};
  const isNew = v.id === '';

  const field = (label, input) => el('label', {}, label, input);
  const text = (value, onchange, max) => { const i = el('input', { type: 'text', maxlength: max, onchange: (e) => onchange(e.target.value) }); i.value = value || ''; return i; };
  const area = (value, onchange, max) => { const i = el('textarea', { rows: 3, maxlength: max, onchange: (e) => onchange(e.target.value) }); i.value = value || ''; return i; };
  const num = (value, onchange) => el('input', { type: 'number', value: value ?? 0, onchange: (e) => onchange(Number(e.target.value)) });
  const boardSelect = el('select', { onchange: (e) => { n.board_id = e.target.value; } }, ...boardOptions().map(([v2, l]) => el('option', { value: v2 }, l)));
  boardSelect.value = n.board_id;
  const raceSelect = el('select', { onchange: (e) => { n.race_id = e.target.value; } }, ...raceOptions(true).map(([v2, l]) => el('option', { value: v2 }, l)));
  raceSelect.value = n.race_id || '';
  const spriteSelect = el('select', { onchange: (e) => { n.sprite = e.target.value; renderPanel(); } },
    el('option', { value: '' }, '(predefinito: wanderer)'), ...state.def.sprites.map((k) => el('option', { value: k }, k)));
  spriteSelect.value = n.sprite || '';
  const traits = el('input', { type: 'text', value: traitsToText(n.traits), placeholder: 'nome=valore, ...' });
  traits.addEventListener('change', () => {
    try { n.traits = textToTraits(traits.value); traits.style.borderColor = ''; notice(''); }
    catch (e) { traits.style.borderColor = 'var(--bad)'; notice(e.message, 'bad'); }
  });

  const actions = [el('button', { onclick: () => saveNpc(isNew) }, isNew ? 'Crea NPC' : 'Salva NPC')];
  if (!isNew) actions.push(el('button', { class: 'danger', onclick: () => deleteNpc(n) }, 'Elimina NPC'));
  return [
    el('div', { class: 'crumb' }, backButton('NPC', () => { state.view = null; notice(''); renderPanel(); }), el('h2', {}, isNew ? 'Nuovo NPC' : n.name)),
    el('div', { class: 'card', style: 'display:flex;flex-direction:column;gap:14px;max-width:820px' },
      el('div', { style: 'display:flex;gap:16px;align-items:flex-end;flex-wrap:wrap' },
        spritePreview(n.sprite || 'wanderer', 64),
        field('Sprite', spriteSelect)),
      el('div', { class: 'grid2' }, field('Nome', text(n.name, (v2) => { n.name = v2; }, 40)), field('Descrizione', area(n.description, (v2) => { n.description = v2; }, 300))),
      el('div', { class: 'grid2' }, field('Board', boardSelect), field('Razza (facoltativa)', raceSelect)),
      el('div', { class: 'grid2' },
        el('div', { class: 'grid2' }, field('x', num(n.x, (v2) => { n.x = v2; })), field('y', num(n.y, (v2) => { n.y = v2; }))),
        el('div', { class: 'grid2' }, field('Velocità', num(n.speed, (v2) => { n.speed = v2; })), field('Vista', num(n.vision, (v2) => { n.vision = v2; })))),
      el('div', { class: 'grid2' }, field('Vita', num(n.health, (v2) => { n.health = v2; })), field('Forza', num(n.strength, (v2) => { n.strength = v2; }))),
      field('Dialogo (una battuta per riga)', area(n.dialogue, (v2) => { n.dialogue = v2; }, 1000)),
      field('Caratteristiche', traits),
      el('div', { class: 'actions' }, ...actions)),
  ];
}

async function saveNpc(isNew) {
  const n = clone(state.view.draft);
  const items = clone(state.def.npcs);
  if (isNew) items.push(n);
  else items.splice(items.findIndex((x) => x.id === state.view.id), 1, n);
  if (await save('/npcs', { items })) {
    state.view = isNew ? null : { type: 'npc', id: n.id, draft: clone(state.def.npcs.find((x) => x.id === n.id)) };
    renderPanel();
  }
}

async function deleteNpc(n) {
  if (!confirm('Eliminare l\'NPC "' + n.name + '"?')) return;
  const items = clone(state.def.npcs).filter((x) => x.id !== n.id);
  if (await save('/npcs', { items })) { state.view = null; renderPanel(); }
}

// -- Oggetti: elenco ---------------------------------------------------------------------------

function renderItemList() {
  const rows = state.def.items.map((it) => el('tr', {},
    el('td', {}, el('strong', {}, it.name)),
    el('td', { class: 'muted' }, (it.description || '').length > 60 ? it.description.slice(0, 60) + '…' : it.description),
    el('td', {}, (state.def.boards.find((b) => b.id === it.board_id) || {}).name || '?'),
    el('td', { class: 'num' }, it.x + ', ' + it.y),
    el('td', {}, it.icon || '(auto)'),
    el('td', {}, el('button', { class: 'ghost', onclick: () => { state.view = { type: 'item', id: it.id, draft: clone(it) }; renderPanel(); } }, 'Apri'))));
  return [
    el('h2', {}, 'Oggetti (' + state.def.items.length + ')'),
    el('p', { class: 'muted' }, 'Oggetti a terra. Chi li raccoglie li mette nell\'inventario della squadra; il campione può usarli. Apri un oggetto per vederlo e modificarlo, oppure vai alla pagina Board per spostarlo sulla mappa.'),
    el('div', { class: 'tablewrap' }, el('table', { class: 'list' },
      el('thead', {}, el('tr', {}, ...['Nome', 'Descrizione', 'Board', 'Casella', 'Icona', ''].map((h) => el('th', {}, h)))),
      el('tbody', {}, rows))),
    el('div', { class: 'actions' },
      el('button', { class: 'ghost', onclick: () => { state.view = { type: 'item', id: '', draft: newItem() }; renderPanel(); } }, '+ Nuovo oggetto')),
  ];
}

function newItem() {
  return { id: '', board_id: state.def.boards[0].id, name: 'Nuovo oggetto', description: '', x: 0, y: 0, effect: {}, icon: '' };
}

// -- Oggetti: dettaglio ------------------------------------------------------------------------

function renderItemDetail() {
  const v = state.view;
  const it = v.draft;
  it.effect = it.effect || {};
  it.effect.traits = it.effect.traits || {};
  const isNew = v.id === '';

  const field = (label, input) => el('label', {}, label, input);
  const text = (value, onchange, max) => { const i = el('input', { type: 'text', maxlength: max, onchange: (e) => onchange(e.target.value) }); i.value = value || ''; return i; };
  const area = (value, onchange, max) => { const i = el('textarea', { rows: 3, maxlength: max, onchange: (e) => onchange(e.target.value) }); i.value = value || ''; return i; };
  const num = (value, onchange) => el('input', { type: 'number', value: value ?? 0, onchange: (e) => onchange(Number(e.target.value)) });
  const boardSelect = el('select', { onchange: (e) => { it.board_id = e.target.value; } }, ...boardOptions().map(([v2, l]) => el('option', { value: v2 }, l)));
  boardSelect.value = it.board_id;
  const iconSelect = el('select', { onchange: (e) => { it.icon = e.target.value; } },
    el('option', { value: '' }, '(automatica, secondo l\'effetto)'), ...state.def.item_icons.map((k) => el('option', { value: k }, k)));
  iconSelect.value = it.icon || '';
  const traits = el('input', { type: 'text', value: traitsToText(it.effect.traits), placeholder: 'nome=valore, ...' });
  traits.addEventListener('change', () => {
    try { it.effect.traits = textToTraits(traits.value); traits.style.borderColor = ''; notice(''); }
    catch (e) { traits.style.borderColor = 'var(--bad)'; notice(e.message, 'bad'); }
  });

  const actions = [el('button', { onclick: () => saveItem(isNew) }, isNew ? 'Crea oggetto' : 'Salva oggetto')];
  if (!isNew) actions.push(el('button', { class: 'danger', onclick: () => deleteItem(it) }, 'Elimina oggetto'));
  return [
    el('div', { class: 'crumb' }, backButton('Oggetti', () => { state.view = null; notice(''); renderPanel(); }), el('h2', {}, isNew ? 'Nuovo oggetto' : it.name)),
    el('div', { class: 'card', style: 'display:flex;flex-direction:column;gap:14px;max-width:760px' },
      el('div', { class: 'grid2' }, field('Nome', text(it.name, (v2) => { it.name = v2; }, 40)), field('Descrizione', area(it.description, (v2) => { it.description = v2; }, 300))),
      el('div', { class: 'grid2' }, field('Board', boardSelect), field('Icona (nell\'inventario)', iconSelect)),
      el('div', { class: 'grid2' }, field('x', num(it.x, (v2) => { it.x = v2; })), field('y', num(it.y, (v2) => { it.y = v2; }))),
      el('h3', { style: 'margin:0' }, 'Effetto (per quando il campione lo usa)'),
      el('div', { class: 'grid2' }, field('Cura', num(it.effect.heal, (v2) => { it.effect.heal = v2; })), field('Punti', num(it.effect.points, (v2) => { it.effect.points = v2; }))),
      el('div', { class: 'grid2' }, field('Forza', num(it.effect.strength, (v2) => { it.effect.strength = v2; })), field('Caratteristiche', traits)),
      el('p', { class: 'muted', style: 'margin:0' }, 'Con tutti gli effetti a zero l\'oggetto è solo decorativo (non si può usare).'),
      el('div', { class: 'actions' }, ...actions)),
  ];
}

async function saveItem(isNew) {
  const it = clone(state.view.draft);
  const items = clone(state.def.items);
  if (isNew) items.push(it);
  else items.splice(items.findIndex((x) => x.id === state.view.id), 1, it);
  if (await save('/items', { items })) {
    state.view = isNew ? null : { type: 'item', id: it.id, draft: clone(state.def.items.find((x) => x.id === it.id)) };
    renderPanel();
  }
}

async function deleteItem(it) {
  if (!confirm('Eliminare l\'oggetto "' + it.name + '"?')) return;
  const items = clone(state.def.items).filter((x) => x.id !== it.id);
  if (await save('/items', { items })) { state.view = null; renderPanel(); }
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
