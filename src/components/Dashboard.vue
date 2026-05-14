<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ProcessTable from './ProcessTable.vue';
import HistoricalView from './HistoricalView.vue';
import SettingsPanel from './SettingsPanel.vue';
import SessionFooter from './SessionFooter.vue';

const tab = ref('live');
const processes = ref([]);
const interfaces = ref([]);
const config = ref({ interface_aliases: {}, process_aliases: {}, hotkey: 'CmdOrCtrl+Shift+N', autostart: false, pip_enabled: false });
const showSettings = ref(false);
const error = ref('');
const toast = ref('');
let toastTimer = null;
const unlisteners = [];

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
  await invoke('start_nethogs_stream').catch((e) => { error.value = String(e); });
});

onUnmounted(() => { unlisteners.forEach((u) => u && u()); });
</script>

<template>
  <div class="dashboard">
    <header class="row app-header">
      <img src="/icon.png" alt="" class="app-logo" />
      <h2 style="margin:0;">JackyNet</h2>
      <div class="spacer"></div>
      <button @click="manualRefresh">Refresh</button>
      <button @click="togglePip">PIP</button>
      <button @click="showSettings = !showSettings">Settings</button>
    </header>

    <transition name="fade">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </transition>

    <div v-if="error" class="panel" style="border-color: var(--danger); color: var(--danger);">
      {{ error }}
    </div>

    <SettingsPanel
      v-if="showSettings"
      :config="config"
      @save="saveSettings"
      @close="showSettings = false"
    />

    <div class="row" style="margin-bottom: 10px;">
      <button :class="{ primary: tab==='live' }" @click="tab='live'">Live Processes</button>
      <button :class="{ primary: tab==='history' }" @click="tab='history'">Historical Usage</button>
    </div>

    <ProcessTable
      v-if="tab==='live'"
      :processes="processes"
      :aliases="config.process_aliases"
      @kill="killProc"
      @rename="(key, name) => setAlias('process', key, name)"
    />

    <HistoricalView
      v-else
      :interfaces="interfaces"
      :aliases="config.interface_aliases"
      @rename="(key, name) => setAlias('interface', key, name)"
    />

    <SessionFooter v-if="tab==='live'" />

    <div class="app-credits">by jaacker25 2026 v1.0</div>
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
