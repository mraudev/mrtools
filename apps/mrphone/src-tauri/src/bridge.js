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
    openLog: () => invoke('open_data_dir'),
    logHeadset: () => {},
    getUpdate: () => Promise.resolve(null),
    installUpdate: () => Promise.resolve(null),
    onUpdate: () => {},
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
})();
