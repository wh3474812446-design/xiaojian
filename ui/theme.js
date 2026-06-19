// theme.js —— 皮肤(主题)系统
// 在首屏渲染前同步应用 data-theme，避免主题闪烁(FOUC)。
// CSP 为 script-src 'self'，不能用内联脚本，故以外部同源脚本实现，并在 <head> 中同步加载。
// 主窗口与候选面板同源、共享 localStorage；切肤通过 storage 事件跨窗口实时同步。
(function () {
  'use strict';

  var KEY = 'xiaojian.theme';
  var DEFAULT = 'obsidian'; // 默认皮肤：曜石(深色实体)

  // 皮肤清单(顺序即皮肤页展示顺序)
  var SKINS = [
    { id: 'obsidian', name: '曜石', desc: '深色实体面板，原生桌面质感' },
    { id: 'glass',    name: '晶透', desc: '半透明毛玻璃，科技氛围光感' },
  ];
  var IDS = SKINS.map(function (s) { return s.id; });

  function normalize(v) { return IDS.indexOf(v) >= 0 ? v : DEFAULT; }

  function read() {
    var v = null;
    try { v = localStorage.getItem(KEY); } catch (e) { /* localStorage 不可用时回退默认 */ }
    return normalize(v);
  }

  function apply(id) {
    document.documentElement.setAttribute('data-theme', normalize(id));
  }

  // —— 首屏同步应用(本脚本在 <head> 同步加载，此时 <html> 已存在) ——
  apply(read());

  var listeners = [];
  function emit(id) { for (var i = 0; i < listeners.length; i++) { try { listeners[i](id); } catch (e) {} } }

  function set(id) {
    id = normalize(id);
    try { localStorage.setItem(KEY, id); } catch (e) {}
    apply(id);
    emit(id);
  }

  // 跨窗口同步：另一窗口改了 localStorage → 本窗口收到 storage 事件
  window.addEventListener('storage', function (e) {
    if (e.key === KEY) { var id = read(); apply(id); emit(id); }
  });

  window.XJTheme = {
    KEY: KEY,
    skins: SKINS,
    get: read,
    set: set,
    apply: apply,
    onChange: function (fn) { if (typeof fn === 'function') listeners.push(fn); },
  };
})();
