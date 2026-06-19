'use strict';

let recent = [];
let favorites = [];
let results = null; // null=未搜索（显示分组）；数组=搜索结果
let candidates = []; // 当前可选项的扁平数组（与 DOM 顺序一致），用于键盘导航
let selectedIndex = 0;
let busy = false;
let searchTimer = null;

const bodyEl = document.getElementById('panel-body');
const emptyEl = document.getElementById('panel-empty');
const toastEl = document.getElementById('panel-toast');
const searchEl = document.getElementById('panel-search');

// 收到主进程推送的候选数据（每次唤起都会发）
window.panelApi.onData((data) => {
  // 每次唤起时同步皮肤(隐藏窗口可能收不到 storage 事件，故主动对齐)
  if (window.XJTheme) window.XJTheme.apply(window.XJTheme.get());
  recent = (data && data.recent) || [];
  favorites = (data && data.favorites) || [];
  results = null;
  selectedIndex = 0;
  busy = false;
  searchEl.value = '';
  hideToast();
  render();
  setTimeout(() => searchEl.focus(), 20);
});

// 搜索输入（防抖 200ms）
searchEl.addEventListener('input', () => {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(doSearch, 200);
});

async function doSearch() {
  const kw = searchEl.value.trim();
  if (!kw) {
    results = null;
    selectedIndex = 0;
    render();
    return;
  }
  const res = await window.panelApi.search(kw);
  results = Array.isArray(res) ? res : [];
  selectedIndex = 0;
  render();
}

function render() {
  bodyEl.innerHTML = '';
  candidates = [];

  if (results !== null) {
    renderGroup('搜索结果', results);
    toggleEmpty(results.length === 0, '未找到相关记录');
  } else {
    if (recent.length) renderGroup('最近', recent);
    if (favorites.length) renderGroup('收藏', favorites);
    toggleEmpty(recent.length === 0 && favorites.length === 0, '暂无可复制记录');
  }
  updateActive();
}

function toggleEmpty(show, text) {
  emptyEl.hidden = !show;
  emptyEl.textContent = text;
  bodyEl.hidden = show;
}

function renderGroup(label, items) {
  const labelEl = document.createElement('div');
  labelEl.className = 'panel-group-label';
  labelEl.textContent = label;
  bodyEl.appendChild(labelEl);

  const ul = document.createElement('ul');
  ul.className = 'panel-list';
  for (const rec of items) {
    const index = candidates.length;
    candidates.push(rec);

    const li = document.createElement('li');
    li.className = 'cand';
    li.dataset.index = index;

    if (rec.type === 'image') {
      const thumb = document.createElement('img');
      thumb.className = 'cand-thumb';
      thumb.src = rec.thumb;
      thumb.alt = '图片';
      li.appendChild(thumb);

      const text = document.createElement('span');
      text.className = 'cand-text';
      text.textContent = rec.preview || '图片';
      li.appendChild(text);

      const chars = document.createElement('span');
      chars.className = 'cand-chars';
      chars.textContent = `${rec.width}×${rec.height}`;
      li.appendChild(chars);
    } else {
      const text = document.createElement('span');
      text.className = 'cand-text';
      text.textContent = rec.preview || rec.content;
      li.appendChild(text);

      if (rec.is_favorite) {
        const star = document.createElement('span');
        star.className = 'cand-star';
        star.textContent = '★';
        li.appendChild(star);
      }

      const chars = document.createElement('span');
      chars.className = 'cand-chars';
      chars.textContent = `${rec.char_count} 字符`;
      li.appendChild(chars);
    }

    li.addEventListener('mouseenter', () => { selectedIndex = index; updateActive(); });
    li.addEventListener('click', () => choose(index));
    ul.appendChild(li);
  }
  bodyEl.appendChild(ul);
}

function allCandEls() {
  return bodyEl.querySelectorAll('.cand');
}

function updateActive() {
  allCandEls().forEach((li) => {
    const active = Number(li.dataset.index) === selectedIndex;
    li.classList.toggle('active', active);
    if (active) li.scrollIntoView({ block: 'nearest' });
  });
}

function move(delta) {
  if (candidates.length === 0) return;
  selectedIndex = (selectedIndex + delta + candidates.length) % candidates.length;
  updateActive();
}

async function choose(index) {
  if (busy) return;
  const rec = candidates[index];
  if (!rec) return;
  busy = true;
  const res = await window.panelApi.select(rec.record_id);
  if (res && res.ok) return; // 主进程已隐藏面板并执行粘贴
  busy = false;
  if (res && res.reason === 'not_found') showToast('记录不存在，列表已刷新');
  else showToast('粘贴失败，请重新选择');
}

// 键盘操作（搜索框聚焦时事件冒泡到 window，仍可用方向键/回车/Esc）
window.addEventListener('keydown', (e) => {
  switch (e.key) {
    case 'Escape':
      e.preventDefault();
      window.panelApi.close();
      break;
    case 'ArrowDown':
      e.preventDefault();
      move(1);
      break;
    case 'ArrowUp':
      e.preventDefault();
      move(-1);
      break;
    case 'Enter':
      e.preventDefault();
      choose(selectedIndex);
      break;
  }
});

let toastTimer = null;
function showToast(text) {
  toastEl.textContent = text;
  toastEl.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(hideToast, 1800);
}
function hideToast() { toastEl.hidden = true; }
