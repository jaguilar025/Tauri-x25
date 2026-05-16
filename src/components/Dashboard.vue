<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ProcessTable from './ProcessTable.vue';
import HistoricalView from './HistoricalView.vue';
import SettingsPanel from './SettingsPanel.vue';
import SessionFooter from './SessionFooter.vue';
import AlertsTab from './AlertsTab.vue';
import AlertIndicators from './AlertIndicators.vue';

const tab = ref('live');
const processes = ref([]);
const interfaces = ref([]);          // vnstat (historical)
const liveInterfaces = ref([]);      // /sys session updates
const config = ref({ interface_aliases: {}, process_aliases: {}, hotkey: 'CmdOrCtrl+Shift+N', autostart: false, pip_enabled: false, alerts: [], alert_iface: null });
const error = ref('');
const toast = ref('');
let toastTimer = null;
const unlisteners = [];

const DISPLAY_MODES = ['auto', 'B/s', 'KB/s', 'MB/s', 'GB/s'];
const displayMode = ref('auto');
function cycleMode() {
  const i = DISPLAY_MODES.indexOf(displayMode.value);
  displayMode.value = DISPLAY_MODES[(i + 1) % DISPLAY_MODES.length];
}

function showToast(msg) {
  toast.value = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { toast.value = ''; }, 2000);
}

async function loadConfig() {
  try { config.value = await invoke('get_config'); } catch (e) { error.value = String(e); }
}

async function refreshHistorical() {
  try { interfaces.value = await invoke('get_vnstat'); error.value = ''; }
  catch (e) { error.value = String(e); }
}

async function refreshLive() {
  try { processes.value = await invoke('nethogs_snapshot'); error.value = ''; }
  catch (e) { error.value = String(e); }
}

async function manualRefresh() {
  await Promise.all([refreshLive(), refreshHistorical()]);
  if (!error.value) showToast('Data refreshed');
}

async function killProc(pid) {
  if (!confirm(`Kill process PID ${pid}?`)) return;
  try { await invoke('kill_process', { pid }); await refreshLive(); }
  catch (e) { error.value = String(e); }
}

async function setAlias(kind, key, name) {
  await invoke('set_alias', { kind, key, name });
  await loadConfig();
}

async function togglePip() {
  await invoke('toggle_pip');
  await loadConfig();
}

async function saveSettings(updated) {
  await invoke('save_config', { config: updated });
  await loadConfig();
}

onMounted(async () => {
  await loadConfig();
  await manualRefresh();
  unlisteners.push(await listen('nethogs:update', (e) => { processes.value = e.payload; }));
  unlisteners.push(await listen('tray:refresh', () => { manualRefresh(); }));
  unlisteners.push(await listen('session:update', (e) => { liveInterfaces.value = e.payload || []; }));
  unlisteners.push(await listen('config:update', (e) => { config.value = e.payload; }));
  try { liveInterfaces.value = await invoke('get_session_stats'); } catch (_) {}
  await invoke('start_nethogs_stream').catch((e) => { error.value = String(e); });
});

onUnmounted(() => { unlisteners.forEach((u) => u && u()); });
</script>

<template>
  <div class="dashboard">
    <header class="row app-header">
      <img src="/icon.png" alt="" class="app-logo" />
      <h2 style="margin:0;">JackyNet</h2>
      <AlertIndicators />
      <div class="spacer"></div>
      <button @click="manualRefresh">Refresh</button>
      <button @click="cycleMode" :title="`Display: ${displayMode}`">Mode: {{ displayMode }}</button>
      <button @click="togglePip">PIP</button>
    </header>

    <transition name="fade">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </transition>

    <div v-if="error" class="panel error-banner">
      <span style="flex: 1;">{{ error }}</span>
      <button class="error-close" @click="error = ''" title="Dismiss">✕</button>
    </div>

    <div class="row tab-bar">
      <button :class="{ primary: tab==='live' }" @click="tab='live'">Live Processes</button>
      <button :class="{ primary: tab==='history' }" @click="tab='history'">Historical Usage</button>
      <button :class="{ primary: tab==='alerts' }" @click="tab='alerts'">Alerts</button>
      <button :class="{ primary: tab==='settings' }" @click="tab='settings'">Settings</button>
    </div>

    <ProcessTable
      v-if="tab==='live'"
      :processes="processes"
      :aliases="config.process_aliases"
      :display-mode="displayMode"
      @kill="killProc"
      @rename="(key, name) => setAlias('process', key, name)"
    />

    <HistoricalView
      v-else-if="tab==='history'"
      :interfaces="interfaces"
      :aliases="config.interface_aliases"
      @rename="(key, name) => setAlias('interface', key, name)"
    />

    <AlertsTab
      v-else-if="tab==='alerts'"
      :config="config"
      :active-interfaces="liveInterfaces"
      @changed="loadConfig"
    />

    <SettingsPanel
      v-else-if="tab==='settings'"
      :config="config"
      @save="saveSettings"
      @close="tab='live'"
    />

    <SessionFooter v-if="tab==='live'" :aliases="config.interface_aliases" />

    <div class="app-credits">by jaacker25 2026 v1.1</div>
  </div>
</template>

<style scoped>
.dashboard {
  padding: 14px;
  max-width: 1100px;
  margin: 0 auto;
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}
.dashboard > :nth-last-child(2) { flex: 0 0 auto; }
.app-credits {
  margin-top: auto;
  padding-top: 16px;
  text-align: right;
  color: var(--muted);
  font-size: 11px;
}
.app-header {
  margin-bottom: 14px;
  gap: 10px;
}
.app-logo {
  width: 28px;
  height: 28px;
  object-fit: contain;
  border-radius: 6px;
}
.tab-bar {
  gap: 8px;
  margin-bottom: 10px;
}
.error-banner {
  border-color: var(--danger);
  color: var(--danger);
  display: flex;
  align-items: center;
  gap: 10px;
}
.error-close {
  background: transparent;
  border: 1px solid var(--danger);
  color: var(--danger);
  padding: 2px 8px;
  font-size: 12px;
  line-height: 1;
}
.error-close:hover {
  background: var(--danger);
  color: #fff;
}
.toast {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--ok);
  color: #0b1a2a;
  font-weight: 600;
  padding: 8px 16px;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  z-index: 1000;
  font-size: 13px;
}
.fade-enter-active, .fade-leave-active { transition: opacity 0.25s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
