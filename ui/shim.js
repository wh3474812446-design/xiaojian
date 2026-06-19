// shim.js —— 注入 window.api / window.panelApi,桥接 Electron 旧前端契约 → Tauri invoke / event

(function () {
  function tauriApi() {
    const { invoke } = window.__TAURI__.core;
    const { listen } = window.__TAURI__.event;
    const on = (event) => (cb) => { listen(event, (e) => cb(e.payload)); };
    return {
      getHistory:      () => invoke('get_history'),
      getFavorites:    () => invoke('get_favorites'),
      getBindings:     () => invoke('get_bindings'),
      copyRecord:      (id) => invoke('copy_record', { id }),
      copyText:        (text) => invoke('copy_text', { text }),
      deleteRecord:    (id) => invoke('delete_record', { id }),
      toggleFavorite:  (id) => invoke('toggle_favorite', { id }),
      togglePin:       (id) => invoke('toggle_pin', { id }),
      setBinding:      (key, recordId) => invoke('set_binding', { key, recordId }),
      unbind:          (key) => invoke('unbind', { key }),
      clearHistory:    () => invoke('clear_history'),
      clearFavorites:  () => invoke('clear_favorites'),
      getSettings:     () => invoke('get_settings'),
      saveSettings:    (patch) => invoke('save_settings', { patch }),
      getAiConfig:     () => invoke('get_ai_config'),
      saveAiConfig:    (cfg) => invoke('save_ai_config', { cfg }),
      translateRecord: (id) => invoke('translate_record', { id }),
      getWatchState:   () => invoke('get_watch_state'),
      toggleWatch:     () => invoke('toggle_watch'),
      onHistoryUpdated: on('history-updated'),
      onWatchChanged:   on('watch-changed'),
    };
  }
  function tauriPanelApi() {
    const { invoke } = window.__TAURI__.core;
    const { listen } = window.__TAURI__.event;
    return {
      onData:   (cb) => { listen('panel-data', (e) => cb(e.payload)); },
      select:   (id) => invoke('panel_select', { id }),
      search:   (keyword) => invoke('panel_search', { keyword }),
      close:    () => invoke('panel_close'),
    };
  }

  window.api = tauriApi();
  window.panelApi = tauriPanelApi();
})();
