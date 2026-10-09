// Brücke window.phone für die Tauri-Version: dieselben Funktionen wie src/preload.js der Electron-Version,
// nur über Tauri-Befehle (invoke) und -Ereignisse (listen). Läuft vor den Skripten der Seite.
(() => {
  const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
  const on = (event, cb) => {
    window.__TAURI__.event.listen(event, (e) => cb(e.payload));
  };
  const fromBase64 = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));

  window.phone = {
    platform: 'win32',
    getState: () => invoke('get_state'),
    command: (msg) => invoke('command', { msg }),
    sendAudio: () => {}, // Gespräche folgen in Stufe 3
    getAudio: () => invoke('get_audio'),
    setAudio: (audio) => invoke('set_audio', { audio }),
    getOptions: () => invoke('get_options'),
    setOptions: (options) => invoke('set_options', { options }),
    getAccounts: () => invoke('get_accounts'),
    saveAccount: (data) => invoke('save_account', { data }),
    deleteAccount: (id) => invoke('delete_account', { id }),
    importPhonerLite: () => invoke('not_yet', { what: 'Der PhonerLite-Import' }),
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
    importOutlook: () => invoke('not_yet', { what: 'Der Outlook-Import' }),
    importCsv: () => invoke('not_yet', { what: 'Der CSV-Import' }),
    exportCsv: () => invoke('not_yet', { what: 'Der CSV-Export' }),
    exportBackup: () => invoke('not_yet', { what: 'Die Sicherung' }),
    chooseBackup: () => invoke('not_yet', { what: 'Die Sicherung' }),
    importBackup: () => invoke('not_yet', { what: 'Die Sicherung' }),
    onContacts: (cb) => on('phone:contactsChanged', cb),
    getFavorites: () => invoke('get_favorites'),
    saveFavorites: (list) => invoke('save_favorites', { list }),
    onPresence: (cb) => on('phone:presence', cb),
    getCti: () => Promise.resolve({ accounts: [], phones: {}, conference: null }),
    onCti: (cb) => on('phone:cti', cb),
    getHistory: () => invoke('get_history'),
    clearHistory: () => invoke('clear_history'),
    onHistory: (cb) => on('phone:historyChanged', cb),
    onShowHistory: (cb) => on('phone:showHistory', cb),
    onState: (cb) => on('phone:state', cb),
    onEnded: (cb) => on('phone:ended', cb),
    onAudio: () => {},
    onAudioFormat: () => {},
    onInfo: (cb) => on('phone:info', cb),
  };
})();
