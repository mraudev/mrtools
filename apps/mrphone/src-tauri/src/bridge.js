// Brücke window.phone für die Tauri-Version: dieselben Funktionen wie src/preload.js der Electron-Version,
// nur über Tauri-Befehle (invoke) und -Ereignisse (listen). Läuft vor den Skripten der Seite.
(() => {
  const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
  const on = (event, cb) => {
    window.__TAURI__.event.listen(event, (e) => cb(e.payload));
  };
  const fromBase64 = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
  // Sprache der Gegenstelle: ein binärer Kanal für die ganze Laufzeit (Int16 in der Rate des Codecs).
  const audioListeners = [];
  let audioChannel = null;
  const onAudio = (cb) => {
    audioListeners.push(cb);
    if (audioChannel) return;
    audioChannel = new window.__TAURI__.core.Channel();
    audioChannel.onmessage = (msg) => {
      const bytes = msg instanceof ArrayBuffer ? new Uint8Array(msg) : Uint8Array.from(msg);
      const pcm = new Int16Array(bytes.buffer, bytes.byteOffset, bytes.byteLength >> 1);
      for (const listener of audioListeners) listener(pcm);
    };
    invoke('audio_subscribe', { channel: audioChannel });
  };
  // Mikrofon: 20-ms-Blöcke als Binärpaket, ohne auf die Antwort zu warten.
  const sendAudio = (pcm) => {
    invoke('audio_in', new Uint8Array(pcm.buffer, pcm.byteOffset, pcm.byteLength)).catch(() => {});
  };

  window.phone = {
    platform: 'win32',
    getState: () => invoke('get_state'),
    command: (msg) => invoke('command', { msg }),
    sendAudio,
    getAudio: () => invoke('get_audio'),
    setAudio: (audio) => invoke('set_audio', { audio }),
    getOptions: () => invoke('get_options'),
    setOptions: (options) => invoke('set_options', { options }),
    getAccounts: () => invoke('get_accounts'),
    saveAccount: (data) => invoke('save_account', { data }),
    deleteAccount: (id) => invoke('delete_account', { id }),
    importPhonerLite: () => invoke('import_phonerlite'),
    getRingtone: async () => {
      const r = await invoke('get_ringtone');
      return r ? { name: r.name, data: fromBase64(r.data) } : null;
    },
    chooseRingtone: () => invoke('choose_ringtone'),
    resetRingtone: () => invoke('reset_ringtone'),
    getVersion: () => invoke('get_version'),
    openLog: () => invoke('open_log'),
    logHeadset: (text) => {
      invoke('log_headset', { text: String(text) }).catch(() => {});
    },
    getUpdate: () => invoke('get_update'),
    installUpdate: () => invoke('install_update'),
    onUpdate: (cb) => on('phone:update', cb),
    getContacts: () => invoke('get_contacts'),
    saveContact: (data) => invoke('save_contact', { data }),
    deleteContact: (id) => invoke('delete_contact', { id }),
    importOutlook: () => invoke('import_outlook'),
    importCsv: () => invoke('import_csv'),
    exportCsv: () => invoke('export_csv'),
    exportBackup: (password) => invoke('export_backup', { password: String(password || '') }),
    chooseBackup: () => invoke('choose_backup'),
    importBackup: (password) => invoke('import_backup', { password: String(password || '') }),
    onContacts: (cb) => on('phone:contactsChanged', cb),
    getFavorites: () => invoke('get_favorites'),
    saveFavorites: (list) => invoke('save_favorites', { list }),
    onPresence: (cb) => on('phone:presence', cb),
    getCti: () => invoke('get_cti'),
    onCti: (cb) => on('phone:cti', cb),
    getHistory: () => invoke('get_history'),
    clearHistory: () => invoke('clear_history'),
    onHistory: (cb) => on('phone:historyChanged', cb),
    onShowHistory: (cb) => on('phone:showHistory', cb),
    onState: (cb) => on('phone:state', cb),
    onEnded: (cb) => on('phone:ended', cb),
    onAudio,
    onAudioFormat: (cb) => on('phone:audioFormat', cb),
    onInfo: (cb) => on('phone:info', cb),
  };

  // Eigene Titelleiste: Die Kopfzeile der Oberfläche zieht das Fenster (app-region: drag, wie in der
  // Electron-Version). Die Fenster-Knöpfe zeichnet in Electron Windows selbst – hier diese Brücke, an
  // derselben Stelle: im klassischen Design im Stil von Windows 11 (Schrift Segoe Fluent Icons), im Design
  // „mrtools“ wie WindowControls aus packages/ui (Lucide-Symbole). Minimieren und Schließen legen ins Tray.
  const GLYPHS = { minimize: '\uE921', maximize: '\uE922', restore: '\uE923', close: '\uE8BB' };
  const LUCIDE = {
    minimize: '<path d="M5 12h14"/>',
    maximize: '<rect width="18" height="18" x="3" y="3" rx="2"/>',
    restore: '<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>',
    close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  };
  const addWindowControls = () => {
    const style = document.createElement('style');
    style.textContent = `
      html .topbar { margin-right: 122px; }
      .window-controls { position: fixed; top: 0; right: 0; z-index: 10; display: flex; height: 44px; -webkit-app-region: no-drag; }
      .window-controls button { display: grid; place-items: center; width: 46px; height: 100%; border: 0; border-radius: 0; padding: 0; margin: 0;
        background: transparent; color: var(--text); font: 10px 'Segoe Fluent Icons', 'Segoe MDL2 Assets'; cursor: default; }
      .window-controls button:hover { background: var(--overlay); }
      .window-controls button:active { background: var(--overlay-strong); }
      .window-controls button.close:hover { background: #c42b1c; color: #fff; }
      .window-controls button.close:active { background: #c84031; color: #fff; }
      .window-controls svg { display: none; width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 2;
        stroke-linecap: round; stroke-linejoin: round; }
      [data-design="mr"] .window-controls { border-left: 1px solid var(--line); }
      [data-design="mr"] .window-controls button { width: 44px; color: var(--muted); transition: background 0.15s, color 0.15s; }
      [data-design="mr"] .window-controls button:hover { background: var(--overlay); color: var(--text); }
      [data-design="mr"] .window-controls button.close:hover, [data-design="mr"] .window-controls button.close:active { background: #dc2626; color: #fff; }
      [data-design="mr"] .window-controls .glyph { display: none; }
      [data-design="mr"] .window-controls svg { display: block; }
      [data-design="mr"] .window-controls .maximize svg { width: 14px; height: 14px; }
      [data-design="mr"] .window-controls .maximize.restored svg { transform: scaleX(-1); }`;
    document.head.append(style);
    const bar = document.createElement('div');
    bar.className = 'window-controls';
    const icon = (b, action) => {
      b.innerHTML = `<span class="glyph">${GLYPHS[action]}</span><svg viewBox="0 0 24 24" aria-hidden="true">${LUCIDE[action]}</svg>`;
    };
    const button = (action, label) => {
      const b = document.createElement('button');
      b.className = action;
      b.title = label;
      b.setAttribute('aria-label', label);
      icon(b, action);
      b.onclick = async () => setMaximized(await invoke('window_control', { action }));
      return b;
    };
    const max = button('maximize', 'Maximieren');
    const setMaximized = (on) => {
      icon(max, on ? 'restore' : 'maximize');
      max.classList.toggle('restored', on);
      max.title = on ? 'Verkleinern' : 'Maximieren';
      max.setAttribute('aria-label', max.title);
    };
    bar.append(button('minimize', 'Minimieren'), max, button('close', 'Schließen'));
    document.body.append(bar);
    // Maximieren per Doppelklick auf die Titelleiste oder Windows-Taste: Symbol nachziehen
    window.addEventListener('resize', async () => setMaximized(await invoke('window_maximized')));
  };
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', addWindowControls);
  else addWindowControls();
})();
