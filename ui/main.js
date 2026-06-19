'use strict';

const DEFAULT_HOTKEY = 'CommandOrControl+Shift+V';
const HOTKEY_KEYS = ['F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11'];

// 已保存设置 & 表单暂存
let savedSettings = { global_hotkey: DEFAULT_HOTKEY, max_records: 100, launch_at_login: false, minimize_to_tray: true };
let form = { ...savedSettings };
let recording = false;

// 数据缓存
let fullHistory = [];
let historyCount = 0;
let favoriteCount = 0;
let maxRecords = 100;
let favData = { pinned: [], normal: [] };
let bindingsData = {};

// 视图与筛选状态
let currentView = 'history';
let searchTerm = '';
let typeFilter = 'all'; // all | text | image
let favSearchTerm = '';

// 选择弹窗状态
let pickerForKey = null;
let pickerSelectedId = null;
let hkpickerRecordId = null;

// AI 翻译状态
let aiEnabled = false; // 已启用且已配置 Key 时为 true，决定是否显示「翻译」按钮
let lastTranslation = '';

const $ = (id) => document.getElementById(id);

document.addEventListener('DOMContentLoaded', async () => {
  bindEvents();
  await loadHistory(true);
  await loadSettings();
  await loadAiConfig();
  renderWatch(await window.api.getWatchState());

  window.api.onHistoryUpdated((data) => {
    applyHistoryData(data);
    refreshCurrentView();
  });
  window.api.onWatchChanged((state) => renderWatch(state));
});

// ============== 事件绑定 ==============
function bindEvents() {
  // 侧边栏导航
  $('side-nav').querySelectorAll('.nav-item').forEach((b) => {
    b.addEventListener('click', () => switchView(b.dataset.view));
  });
  $('watch-banner').addEventListener('click', toggleWatch);

  // 自绘标题栏窗口控制(无边框窗口；浏览器预览无 __TAURI__ 时静默跳过)
  const winNs = window.__TAURI__ && window.__TAURI__.window;
  if (winNs && winNs.getCurrentWindow) {
    const appWin = winNs.getCurrentWindow();
    $('win-min').addEventListener('click', () => appWin.minimize());
    $('win-close').addEventListener('click', () => appWin.close());
  }

  // 历史页搜索
  $('search-input').addEventListener('input', (e) => {
    searchTerm = e.target.value.trim().toLowerCase();
    $('search-clear').hidden = !e.target.value;
    applyAndRenderHistory();
  });
  $('search-clear').addEventListener('click', () => {
    $('search-input').value = '';
    searchTerm = '';
    $('search-clear').hidden = true;
    applyAndRenderHistory();
    $('search-input').focus();
  });

  // 历史页分类标签
  $('filter-tabs').querySelectorAll('button').forEach((b) => {
    b.addEventListener('click', () => {
      typeFilter = b.dataset.type;
      $('filter-tabs').querySelectorAll('button').forEach((x) => x.classList.toggle('active', x === b));
      applyAndRenderHistory();
    });
  });

  // 收藏页搜索
  $('fav-search-input').addEventListener('input', (e) => {
    favSearchTerm = e.target.value.trim().toLowerCase();
    $('fav-search-clear').hidden = !e.target.value;
    renderFavorites();
  });
  $('fav-search-clear').addEventListener('click', () => {
    $('fav-search-input').value = '';
    favSearchTerm = '';
    $('fav-search-clear').hidden = true;
    renderFavorites();
    $('fav-search-input').focus();
  });

  // 设置：历史容量
  $('seg-max').querySelectorAll('button').forEach((b) => {
    b.addEventListener('click', () => {
      form.max_records = Number(b.dataset.v);
      renderSegMax();
      checkDirty();
    });
  });
  // 设置：开机自启 / 托盘
  $('chk-launch').addEventListener('change', (e) => { form.launch_at_login = e.target.checked; checkDirty(); });
  $('chk-tray').addEventListener('change', (e) => { form.minimize_to_tray = e.target.checked; checkDirty(); });
  // 设置：快捷键
  $('btn-edit-hotkey').addEventListener('click', startRecordHotkey);
  $('btn-reset-hotkey').addEventListener('click', () => {
    stopRecordHotkey();
    form.global_hotkey = DEFAULT_HOTKEY;
    renderHotkey();
    checkDirty();
  });
  $('btn-save').addEventListener('click', saveSettings);

  // 清空普通历史
  $('btn-clear').addEventListener('click', openClearModal);
  $('modal-cancel').addEventListener('click', () => ($('clear-modal').hidden = true));
  $('modal-confirm').addEventListener('click', confirmClear);
  $('clear-modal').addEventListener('click', (e) => { if (e.target === $('clear-modal')) $('clear-modal').hidden = true; });

  // 清空全部收藏
  $('btn-clear-fav').addEventListener('click', openClearFavModal);
  $('fav-modal-cancel').addEventListener('click', () => ($('clear-fav-modal').hidden = true));
  $('fav-modal-confirm').addEventListener('click', confirmClearFav);
  $('clear-fav-modal').addEventListener('click', (e) => { if (e.target === $('clear-fav-modal')) $('clear-fav-modal').hidden = true; });

  // 选择记录弹窗（绑定页「更换」）
  $('picker-search-input').addEventListener('input', (e) => renderPickerList(e.target.value.trim().toLowerCase()));
  $('picker-cancel').addEventListener('click', closePicker);
  $('picker-confirm').addEventListener('click', confirmPicker);
  $('record-picker').addEventListener('click', (e) => { if (e.target === $('record-picker')) closePicker(); });

  // 选择快捷键弹窗（历史/收藏页「绑定」）
  $('hkpicker-cancel').addEventListener('click', closeHkpicker);
  $('hotkey-picker').addEventListener('click', (e) => { if (e.target === $('hotkey-picker')) closeHkpicker(); });

  // AI 翻译设置
  $('chk-ai').addEventListener('change', (e) => { $('ai-detail').hidden = !e.target.checked; });
  $('btn-save-ai').addEventListener('click', saveAiSettings);
  // 翻译结果弹窗
  $('tr-close').addEventListener('click', closeTranslate);
  $('tr-copy').addEventListener('click', copyTranslation);
  $('translate-modal').addEventListener('click', (e) => { if (e.target === $('translate-modal')) closeTranslate(); });

  // 快捷键录制：全局键盘监听
  window.addEventListener('keydown', onRecordKeydown, true);
  // Esc 关闭弹窗
  window.addEventListener('keydown', (e) => {
    if (e.key !== 'Escape' || recording) return;
    if (!$('record-picker').hidden) closePicker();
    else if (!$('hotkey-picker').hidden) closeHkpicker();
    else if (!$('clear-modal').hidden) $('clear-modal').hidden = true;
    else if (!$('clear-fav-modal').hidden) $('clear-fav-modal').hidden = true;
    else if (!$('translate-modal').hidden) closeTranslate();
  });
}

// ============== 视图切换 ==============
function switchView(name) {
  currentView = name;
  $('side-nav').querySelectorAll('.nav-item').forEach((b) => b.classList.toggle('active', b.dataset.view === name));
  ['history', 'favorites', 'bindings', 'settings'].forEach((v) => {
    $('view-' + v).classList.toggle('active', v === name);
  });
  if (name === 'favorites') loadFavorites();
  else if (name === 'bindings') loadBindings();
  else if (name === 'settings') { stopRecordHotkey(); loadSettings(); loadAiConfig(); }
}

function refreshCurrentView() {
  if (currentView === 'history') applyAndRenderHistory();
  else if (currentView === 'favorites') loadFavorites();
  else if (currentView === 'bindings') loadBindings();
}

// ============== 历史数据 ==============
async function loadHistory(initial) {
  try {
    applyHistoryData(await window.api.getHistory());
    applyAndRenderHistory();
  } catch (e) {
    if (initial) showToast('历史记录读取失败，请重启应用', true);
  }
}

function applyHistoryData(data) {
  $('loading-state').hidden = true;
  fullHistory = data.history || [];
  historyCount = data.historyCount != null ? data.historyCount : data.count;
  favoriteCount = data.favoriteCount || 0;
  maxRecords = data.max;
  $('stat-count').textContent = `${historyCount} 条历史`;
  $('stat-fav').textContent = `${favoriteCount} 条收藏`;
  const badge = $('nav-fav-badge');
  badge.hidden = favoriteCount === 0;
  badge.textContent = String(favoriteCount);
  $('fav-count').textContent = String(favoriteCount);
}

// 历史页：按搜索 + 分类过滤后渲染
function applyAndRenderHistory() {
  const list = $('history-list');
  const empty = $('empty-state');

  let items = fullHistory;
  if (typeFilter !== 'all') items = items.filter((r) => r.type === typeFilter);
  if (searchTerm) items = items.filter((r) => r.type === 'text' && r.content.toLowerCase().includes(searchTerm));

  if (items.length === 0) {
    list.hidden = true;
    empty.hidden = false;
    $('empty-text').textContent = historyEmptyMessage();
    return;
  }
  empty.hidden = true;
  list.hidden = false;
  list.innerHTML = '';
  for (const rec of items) list.appendChild(renderRecord(rec, 'history'));
}

function historyEmptyMessage() {
  if (fullHistory.length === 0) return '暂无复制记录，复制一段文字后会显示在这里';
  if (searchTerm) return '未找到相关记录';
  if (typeFilter === 'image') return '暂无图片记录，复制一张图片后会显示在这里';
  if (typeFilter === 'text') return '暂无文本记录';
  return '暂无复制记录';
}

// ============== 收藏数据 ==============
async function loadFavorites() {
  favData = await window.api.getFavorites();
  renderFavorites();
}

function renderFavorites() {
  const filt = (arr) =>
    favSearchTerm ? arr.filter((r) => r.type === 'text' && r.content.toLowerCase().includes(favSearchTerm)) : arr;
  const pinned = filt(favData.pinned || []);
  const normal = filt(favData.normal || []);
  const totalRaw = (favData.pinned || []).length + (favData.normal || []).length;

  const empty = $('fav-empty');
  if (pinned.length === 0 && normal.length === 0) {
    $('fav-pinned-section').hidden = true;
    $('fav-normal-section').hidden = true;
    empty.hidden = false;
    empty.querySelector('p').textContent =
      totalRaw === 0 ? '暂无收藏记录，点击星标可收藏重要内容' : '未找到相关收藏记录';
    return;
  }
  empty.hidden = true;

  const pSec = $('fav-pinned-section');
  const pList = $('fav-pinned-list');
  if (pinned.length) {
    pSec.hidden = false;
    pList.innerHTML = '';
    for (const rec of pinned) pList.appendChild(renderRecord(rec, 'favorites'));
  } else pSec.hidden = true;

  const nSec = $('fav-normal-section');
  const nList = $('fav-normal-list');
  if (normal.length) {
    nSec.hidden = false;
    nList.innerHTML = '';
    for (const rec of normal) nList.appendChild(renderRecord(rec, 'favorites'));
  } else nSec.hidden = true;
}

// ============== 记录卡片渲染（历史页 / 收藏页通用） ==============
function renderRecord(rec, ctx) {
  const li = document.createElement('li');
  li.className = 'record' + (rec.type === 'image' ? ' record-image' : '');
  li.dataset.id = rec.record_id;

  const body = document.createElement('div');
  body.className = 'record-body';

  if (rec.type === 'image') {
    const thumb = document.createElement('div');
    thumb.className = 'record-thumb';
    const img = document.createElement('img');
    img.src = rec.thumb;
    img.alt = '剪贴板图片';
    thumb.appendChild(img);
    body.appendChild(thumb);

    const meta = document.createElement('div');
    meta.className = 'record-meta';
    meta.innerHTML =
      `<span class="type-badge img">图片</span>` +
      `<span>${formatTime(rec.copied_at)}</span><span>·</span>` +
      `<span>${rec.width}×${rec.height}</span><span>·</span><span>${formatSize(rec.byte_size)}</span>` +
      (ctx === 'history' && rec.is_favorite ? `<span class="fav-star">★</span>` : '');
    body.appendChild(meta);
  } else {
    const preview = document.createElement('div');
    preview.className = 'record-preview';
    preview.textContent = rec.preview || rec.content;
    body.appendChild(preview);

    const meta = document.createElement('div');
    meta.className = 'record-meta';
    meta.innerHTML =
      `<span class="type-badge txt">文本</span>` +
      `<span>${formatTime(rec.copied_at)}</span><span>·</span><span>${rec.char_count} 字符</span>` +
      (ctx === 'history' && rec.is_favorite ? `<span class="fav-star">★</span>` : '');
    body.appendChild(meta);
  }

  const actions = document.createElement('div');
  actions.className = 'record-actions';

  const copyBtn = mkBtn('copy', '复制', (e) => { e.stopPropagation(); doCopy(rec.record_id, copyBtn); });
  actions.appendChild(copyBtn);

  if (ctx === 'history') {
    const favBtn = mkBtn('star' + (rec.is_favorite ? ' active' : ''), rec.is_favorite ? '已收藏' : '收藏', (e) => {
      e.stopPropagation();
      doToggleFavorite(rec.record_id);
    });
    const bindBtn = mkBtn('bind', '绑定', (e) => { e.stopPropagation(); openHotkeyPicker(rec.record_id); });
    const delBtn = mkBtn('del', '删除', (e) => { e.stopPropagation(); doDelete(rec.record_id); });
    actions.appendChild(favBtn);
    actions.appendChild(bindBtn);
    actions.appendChild(delBtn);
  } else {
    const unfavBtn = mkBtn('star active', '取消收藏', (e) => { e.stopPropagation(); doToggleFavorite(rec.record_id); });
    const pinBtn = mkBtn('pin' + (rec.is_pinned ? ' active' : ''), rec.is_pinned ? '取消置顶' : '置顶', (e) => {
      e.stopPropagation();
      doTogglePin(rec.record_id);
    });
    const bindBtn = mkBtn('bind', '绑定', (e) => { e.stopPropagation(); openHotkeyPicker(rec.record_id); });
    actions.appendChild(unfavBtn);
    actions.appendChild(pinBtn);
    actions.appendChild(bindBtn);
  }

  if (aiEnabled && rec.type === 'text') {
    const trBtn = mkBtn('tr', '翻译', (e) => { e.stopPropagation(); openTranslate(rec); });
    actions.appendChild(trBtn);
  }

  li.appendChild(body);
  li.appendChild(actions);
  li.addEventListener('click', () => doCopy(rec.record_id, copyBtn));
  return li;
}

function mkBtn(cls, text, onClick) {
  const b = document.createElement('button');
  b.className = 'act ' + cls;
  b.textContent = text;
  b.addEventListener('click', onClick);
  return b;
}

// ============== 记录操作 ==============
async function doCopy(id, btn) {
  const res = await window.api.copyRecord(id);
  if (res.ok) {
    if (btn) {
      const old = btn.textContent;
      btn.textContent = '已复制';
      btn.classList.add('done');
      setTimeout(() => { btn.textContent = old; btn.classList.remove('done'); }, 1200);
    }
    showToast('已复制到剪贴板');
  } else if (res.reason === 'not_found') {
    showToast('记录不存在，列表已刷新', true);
  } else {
    showToast('复制失败，请重新尝试', true);
  }
}

async function doDelete(id) {
  await window.api.deleteRecord(id);
}

async function doToggleFavorite(id) {
  const res = await window.api.toggleFavorite(id);
  if (res && res.ok) showToast(res.is_favorite ? '已收藏' : '已取消收藏');
  else if (res && res.reason === 'not_found') showToast('记录不存在，列表已刷新', true);
  else showToast('收藏操作失败，请重试', true);
}

async function doTogglePin(id) {
  const res = await window.api.togglePin(id);
  if (res && res.ok) showToast(res.is_pinned ? '已置顶' : '已取消置顶');
  else if (res && res.reason === 'not_favorite') showToast('请先收藏后再置顶', true);
  else showToast('置顶操作失败，请重试', true);
}

// ============== 快捷绑定页 ==============
async function loadBindings() {
  bindingsData = await window.api.getBindings();
  renderBindings();
}

function renderBindings() {
  const list = $('bindings-list');
  list.innerHTML = '';
  for (const key of HOTKEY_KEYS) {
    const b = bindingsData[key] || { bound: false };
    const li = document.createElement('li');
    li.className = 'binding-item';

    const keyEl = document.createElement('div');
    keyEl.className = 'binding-key';
    keyEl.textContent = 'Ctrl + ' + key;

    const content = document.createElement('div');
    content.className = 'binding-content' + (b.bound ? '' : ' empty');
    if (b.bound) {
      if (b.type === 'image' && b.thumb) {
        const img = document.createElement('img');
        img.className = 'binding-thumb';
        img.src = b.thumb;
        content.appendChild(img);
      }
      const span = document.createElement('span');
      span.textContent = b.type === 'image' ? (b.preview || '图片') : b.preview;
      content.appendChild(span);
    } else {
      content.textContent = '未绑定';
    }

    const acts = document.createElement('div');
    acts.className = 'binding-actions';
    const changeBtn = mkBtn('bind', b.bound ? '更换' : '绑定', () => openRecordPicker(key));
    acts.appendChild(changeBtn);
    if (b.bound) {
      const unbindBtn = mkBtn('del', '解绑', () => doUnbind(key));
      acts.appendChild(unbindBtn);
    }

    li.appendChild(keyEl);
    li.appendChild(content);
    li.appendChild(acts);
    list.appendChild(li);
  }
}

async function doUnbind(key) {
  await window.api.unbind(key);
  showToast(`已解绑 Ctrl + ${key}`);
}

// ============== 选择记录弹窗（绑定页「更换/绑定」） ==============
function openRecordPicker(key) {
  pickerForKey = key;
  pickerSelectedId = null;
  $('picker-title').textContent = `为 Ctrl + ${key} 选择记录`;
  $('picker-search-input').value = '';
  $('picker-confirm').disabled = true;
  renderPickerList('');
  $('record-picker').hidden = false;
  setTimeout(() => $('picker-search-input').focus(), 30);
}

function renderPickerList(term) {
  const list = $('picker-list');
  const empty = $('picker-empty');
  const items = fullHistory.filter((r) => !term || (r.type === 'text' && r.content.toLowerCase().includes(term)));
  list.innerHTML = '';
  if (items.length === 0) {
    list.hidden = true;
    empty.hidden = false;
    empty.textContent = fullHistory.length === 0 ? '暂无可绑定记录，请先复制一段文字' : '未找到相关记录';
    return;
  }
  empty.hidden = true;
  list.hidden = false;
  for (const rec of items) {
    const li = document.createElement('li');
    li.className = 'picker-item' + (rec.record_id === pickerSelectedId ? ' selected' : '');
    li.dataset.id = rec.record_id;
    if (rec.type === 'image' && rec.thumb) {
      const img = document.createElement('img');
      img.className = 'pi-thumb';
      img.src = rec.thumb;
      li.appendChild(img);
    }
    const text = document.createElement('span');
    text.className = 'pi-text';
    text.textContent = rec.type === 'image' ? (rec.preview || '图片') : (rec.preview || rec.content);
    li.appendChild(text);
    const meta = document.createElement('span');
    meta.className = 'pi-meta';
    meta.textContent = rec.type === 'image' ? `${rec.width}×${rec.height}` : `${rec.char_count} 字符`;
    li.appendChild(meta);

    li.addEventListener('click', () => {
      pickerSelectedId = rec.record_id;
      $('picker-confirm').disabled = false;
      list.querySelectorAll('.picker-item').forEach((x) => x.classList.toggle('selected', x.dataset.id === pickerSelectedId));
    });
    list.appendChild(li);
  }
}

async function confirmPicker() {
  if (!pickerForKey || !pickerSelectedId) return;
  const res = await window.api.setBinding(pickerForKey, pickerSelectedId);
  closePicker();
  if (res && res.ok) showToast('绑定已保存');
  else if (res && res.reason === 'record_not_found') showToast('记录不存在，请重新选择', true);
  else showToast('绑定失败，请重试', true);
}

function closePicker() {
  $('record-picker').hidden = true;
  pickerForKey = null;
  pickerSelectedId = null;
}

// ============== 选择快捷键弹窗（历史/收藏页「绑定」） ==============
async function openHotkeyPicker(recordId) {
  hkpickerRecordId = recordId;
  const bindings = await window.api.getBindings();
  const list = $('hkpicker-list');
  list.innerHTML = '';
  for (const key of HOTKEY_KEYS) {
    const b = bindings[key] || { bound: false };
    const li = document.createElement('li');
    li.className = 'hkpicker-item';
    const k = document.createElement('span');
    k.className = 'hkpicker-key';
    k.textContent = 'Ctrl + ' + key;
    const state = document.createElement('span');
    state.className = 'hkpicker-state';
    state.textContent = b.bound ? `已绑定：${b.preview || ''}` : '空闲';
    li.appendChild(k);
    li.appendChild(state);
    li.addEventListener('click', () => chooseHotkey(key));
    list.appendChild(li);
  }
  $('hotkey-picker').hidden = false;
}

async function chooseHotkey(key) {
  if (!hkpickerRecordId) return;
  const res = await window.api.setBinding(key, hkpickerRecordId);
  closeHkpicker();
  if (res && res.ok) showToast(`已绑定到 Ctrl + ${key}`);
  else if (res && res.reason === 'record_not_found') showToast('记录不存在，无法绑定', true);
  else showToast('绑定失败，请重试', true);
}

function closeHkpicker() {
  $('hotkey-picker').hidden = true;
  hkpickerRecordId = null;
}

// ============== 监听状态 ==============
function renderWatch(watching) {
  const b = $('watch-banner');
  b.classList.toggle('paused', !watching);
  $('watch-text').textContent = watching ? '正在监听' : '已暂停';
  $('watch-action').textContent = watching ? '点击暂停' : '点击恢复';
}

async function toggleWatch() {
  renderWatch(await window.api.toggleWatch());
}

// ============== 设置 ==============
async function loadSettings() {
  const s = await window.api.getSettings();
  savedSettings = { ...s };
  form = { ...s };
  renderHotkey();
  renderSegMax();
  $('chk-launch').checked = !!form.launch_at_login;
  $('chk-tray').checked = form.minimize_to_tray !== false;
  checkDirty();
  clearSaveFeedback();
}

function renderHotkey() {
  $('hotkey-display').textContent = accelToLabel(form.global_hotkey);
}

function renderSegMax() {
  $('seg-max').querySelectorAll('button').forEach((b) => {
    b.classList.toggle('active', Number(b.dataset.v) === form.max_records);
  });
}

function checkDirty() {
  const dirty =
    form.global_hotkey !== savedSettings.global_hotkey ||
    form.max_records !== savedSettings.max_records ||
    form.launch_at_login !== savedSettings.launch_at_login ||
    form.minimize_to_tray !== savedSettings.minimize_to_tray;
  $('btn-save').disabled = !dirty;
}

async function saveSettings() {
  $('btn-save').disabled = true;
  setSaveFeedback('正在保存设置…', '');
  const res = await window.api.saveSettings({ ...form });
  if (res.ok) {
    savedSettings = { ...res.settings };
    form = { ...res.settings };
    renderHotkey();
    renderSegMax();
    $('chk-launch').checked = !!form.launch_at_login;
    $('chk-tray').checked = form.minimize_to_tray !== false;
    setSaveFeedback('设置已保存', 'ok');
    setTimeout(clearSaveFeedback, 1500);
  } else if (res.reason === 'invalid_max_records') {
    setSaveFeedback('请选择有效的历史保留数量', 'err');
    $('btn-save').disabled = false;
  } else {
    setSaveFeedback('设置保存失败，请重试', 'err');
    $('btn-save').disabled = false;
  }
}

function setSaveFeedback(text, kind) {
  const el = $('save-feedback');
  el.textContent = text;
  el.className = 'save-feedback' + (kind ? ' ' + kind : '');
}
function clearSaveFeedback() { setSaveFeedback('', ''); }

// ---- 快捷键录制 ----
function startRecordHotkey() {
  recording = true;
  $('hotkey-display').classList.add('recording');
  $('hotkey-display').textContent = '请按下快捷键组合…';
  $('btn-edit-hotkey').textContent = '录制中';
}

function stopRecordHotkey() {
  if (!recording) return;
  recording = false;
  $('hotkey-display').classList.remove('recording');
  $('btn-edit-hotkey').textContent = '修改';
  renderHotkey();
}

function onRecordKeydown(e) {
  if (!recording) return;
  e.preventDefault();
  e.stopPropagation();

  if (e.key === 'Escape') { stopRecordHotkey(); return; }

  const acc = eventToAccelerator(e);
  if (!acc) {
    const mods = [];
    if (e.ctrlKey) mods.push('Ctrl');
    if (e.altKey) mods.push('Alt');
    if (e.shiftKey) mods.push('Shift');
    if (e.metaKey) mods.push('Win');
    $('hotkey-display').textContent = (mods.join(' + ') || '请按下快捷键组合…') + (mods.length ? ' + …' : '');
    return;
  }

  form.global_hotkey = acc;
  recording = false;
  $('hotkey-display').classList.remove('recording');
  $('btn-edit-hotkey').textContent = '修改';
  renderHotkey();
  checkDirty();
}

function eventToAccelerator(e) {
  const parts = [];
  if (e.ctrlKey) parts.push('CommandOrControl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Super');
  const key = normalizeKey(e);
  if (!key) return null;
  if (parts.length === 0) return null;
  parts.push(key);
  return parts.join('+');
}

function normalizeKey(e) {
  const k = e.key;
  if (['Control', 'Shift', 'Alt', 'Meta', 'OS'].includes(k)) return null;
  const code = e.code || '';
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(k)) return k;
  const named = {
    ' ': 'Space', Enter: 'Return', Tab: 'Tab', Backspace: 'Backspace',
    Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End',
    PageUp: 'PageUp', PageDown: 'PageDown',
    ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right',
  };
  if (named[k]) return named[k];
  if (k.length === 1) return k.toUpperCase();
  return null;
}

function accelToLabel(acc) {
  return String(acc)
    .replace('CommandOrControl', 'Ctrl')
    .replace('CmdOrCtrl', 'Ctrl')
    .replace('Control', 'Ctrl')
    .replace('Super', 'Win')
    .replace('Return', 'Enter')
    .split('+')
    .join(' + ');
}

// ============== AI 翻译 ==============
async function loadAiConfig() {
  const cfg = await window.api.getAiConfig();
  const nextEnabled = !!(cfg && cfg.enabled && cfg.hasKey);
  $('chk-ai').checked = !!(cfg && cfg.enabled);
  $('ai-detail').hidden = !(cfg && cfg.enabled);
  if (cfg && cfg.model) $('ai-model').value = cfg.model;
  $('ai-key').value = '';
  $('ai-key').placeholder = cfg && cfg.hasKey ? '已保存 Key（留空则不修改）' : '填入 DeepSeek API Key（sk-...）';
  renderAiStatus(cfg);
  if (nextEnabled !== aiEnabled) {
    aiEnabled = nextEnabled;
    refreshCurrentView(); // 翻译按钮显隐变化，重渲染列表
  }
}

function renderAiStatus(cfg) {
  const el = $('ai-status');
  if (!cfg) { el.textContent = ''; return; }
  const parts = [cfg.hasKey ? 'API Key 已配置' : '尚未配置 API Key'];
  if (cfg.hasKey) parts.push(cfg.encryption ? '本地加密保存' : '本地保存（系统未提供加密）');
  if (cfg.enabled && !cfg.hasKey) parts.push('填入 Key 后才能翻译');
  el.textContent = parts.join(' · ');
}

async function saveAiSettings() {
  const patch = { enabled: $('chk-ai').checked, model: $('ai-model').value };
  const keyVal = $('ai-key').value;
  if (keyVal) patch.apiKey = keyVal; // 留空则不修改已存 Key
  $('btn-save-ai').disabled = true;
  setAiFeedback('正在保存…', '');
  const res = await window.api.saveAiConfig(patch);
  $('btn-save-ai').disabled = false;
  if (res && res.ok) {
    setAiFeedback('已保存', 'ok');
    setTimeout(() => setAiFeedback('', ''), 1500);
    $('ai-key').value = '';
    $('ai-key').placeholder = res.hasKey ? '已保存 Key（留空则不修改）' : '填入 DeepSeek API Key（sk-...）';
    renderAiStatus(res);
    const nextEnabled = !!(res.enabled && res.hasKey);
    if (nextEnabled !== aiEnabled) { aiEnabled = nextEnabled; refreshCurrentView(); }
  } else {
    setAiFeedback('保存失败，请重试', 'err');
  }
}

function setAiFeedback(text, kind) {
  const el = $('ai-feedback');
  el.textContent = text;
  el.className = 'save-feedback' + (kind ? ' ' + kind : '');
}

async function openTranslate(rec) {
  lastTranslation = '';
  $('tr-source').textContent = rec.content || rec.preview || '';
  $('tr-result').className = 'tr-text result';
  $('tr-result').textContent = '正在翻译…';
  $('tr-target-label').textContent = '';
  $('tr-copy').disabled = true;
  $('translate-modal').hidden = false;

  const res = await window.api.translateRecord(rec.record_id);
  if (res && res.ok) {
    $('tr-result').className = 'tr-text result';
    $('tr-result').textContent = res.text;
    $('tr-target-label').textContent = res.target === 'en' ? '（英文）' : '（中文）';
    lastTranslation = res.text;
    $('tr-copy').disabled = false;
  } else {
    $('tr-result').className = 'tr-text tr-error';
    $('tr-result').textContent = translateErrorText(res && res.reason);
    $('tr-copy').disabled = true;
  }
}

function translateErrorText(reason) {
  switch (reason) {
    case 'disabled':
    case 'no_key': return '请先在设置中配置并启用 DeepSeek 翻译';
    case 'not_text': return '该记录不是文本，无法翻译';
    case 'too_long': return '内容过长，暂不支持翻译（上限约 8000 字）';
    case 'empty': return '内容为空，无需翻译';
    case 'unauthorized': return 'API Key 无效或已过期，请在设置中检查';
    case 'timeout': return '翻译超时，请稍后重试';
    case 'network': return '网络错误，请检查网络连接后重试';
    case 'empty_result': return '未获得翻译结果，请重试';
    default: return '翻译失败，请稍后重试';
  }
}

function closeTranslate() {
  $('translate-modal').hidden = true;
  lastTranslation = '';
}

async function copyTranslation() {
  if (!lastTranslation) return;
  const res = await window.api.copyText(lastTranslation);
  if (res && res.ok) showToast('已复制译文');
  else showToast('复制失败，请重试', true);
}

// ============== 清空 ==============
function openClearModal() {
  $('modal-count').textContent = String(historyCount);
  $('clear-modal').hidden = false;
}
async function confirmClear() {
  const res = await window.api.clearHistory();
  $('clear-modal').hidden = true;
  if (res.ok) showToast('已清空普通历史，收藏记录已保留');
  else showToast('清空失败，请重试', true);
}

function openClearFavModal() {
  $('fav-modal-count').textContent = String(favoriteCount);
  $('clear-fav-modal').hidden = false;
}
async function confirmClearFav() {
  const res = await window.api.clearFavorites();
  $('clear-fav-modal').hidden = true;
  if (res.ok) showToast('已清空全部收藏');
  else showToast('清空失败，请重试', true);
}

// ============== 工具 ==============
let toastTimer = null;
function showToast(text, isErr) {
  const t = $('toast');
  t.textContent = text;
  t.className = 'toast' + (isErr ? ' err' : '');
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { t.hidden = true; }, 1800);
}

function formatSize(b) {
  if (!b && b !== 0) return '';
  if (b < 1024) return b + ' B';
  if (b < 1024 * 1024) return (b / 1024).toFixed(0) + ' KB';
  return (b / 1024 / 1024).toFixed(1) + ' MB';
}

function formatTime(ts) {
  const now = Date.now();
  const diff = Math.floor((now - ts) / 1000);
  if (diff < 10) return '刚刚';
  if (diff < 60) return `${diff} 秒前`;
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  const d = new Date(ts);
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const today = new Date();
  const yest = new Date(today);
  yest.setDate(today.getDate() - 1);
  if (d.toDateString() === yest.toDateString()) return `昨天 ${hh}:${mm}`;
  if (d.getFullYear() === today.getFullYear()) {
    return `${d.getMonth() + 1}-${String(d.getDate()).padStart(2, '0')} ${hh}:${mm}`;
  }
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}
