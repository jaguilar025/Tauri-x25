<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const props = defineProps({ compact: { type: Boolean, default: false } });

const stats = ref({ started_at: '', started_unix_ms: 0, total_rx_bytes: 0, total_tx_bytes: 0 });
const now = ref(Date.now());
let timer = null;
let unlisten = null;

function fmtBytes(b) {
  if (!b) return '0 B';
  const u = ['B','KB','MB','GB','TB'];
  let i = 0; let v = Number(b);
  while (v >= 1024 && i < u.length-1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 ? 2 : v < 100 ? 1 : 0)} ${u[i]}`;
}

const MONTHS_ES = ['Enero','Febrero','Marzo','Abril','Mayo','Junio','Julio','Agosto','Septiembre','Octubre','Noviembre','Diciembre'];

const sessionLabel = computed(() => {
  if (!stats.value.started_unix_ms) return '—';
  const d = new Date(stats.value.started_unix_ms);
  const day = d.getDate();
  const month = MONTHS_ES[d.getMonth()];
  const year = d.getFullYear();
  let h = d.getHours();
  const m = d.getMinutes().toString().padStart(2, '0');
  const ampm = h >= 12 ? 'pm' : 'am';
  h = h % 12 || 12;
  return `${day}/${month}/${year} ${h.toString().padStart(2,'0')}:${m}${ampm}`;
});

const onlineLabel = computed(() => {
  if (!stats.value.started_unix_ms) return '0:00:00';
  const secs = Math.max(0, Math.floor((now.value - stats.value.started_unix_ms) / 1000));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60).toString().padStart(2, '0');
  const s = (secs % 60).toString().padStart(2, '0');
  return `${h}:${m}:${s}`;
});

onMounted(async () => {
  try { stats.value = await invoke('get_session_stats'); } catch (_) {}
  unlisten = await listen('nethogs:session', (e) => { stats.value = e.payload; });
  timer = setInterval(() => { now.value = Date.now(); }, 1000);
});
onUnmounted(() => {
  if (timer) clearInterval(timer);
  if (unlisten) unlisten();
});
</script>

<template>
  <div :class="['session-footer', { compact }]">
    <div><span class="muted">Active session:</span> {{ sessionLabel }}</div>
    <div>
      <span class="muted">Accumulated total:</span>
      ↑{{ fmtBytes(stats.total_tx_bytes) }}
      ↓{{ fmtBytes(stats.total_rx_bytes) }}
    </div>
    <div><span class="muted">Online:</span> <span class="mono">{{ onlineLabel }}</span></div>
  </div>
</template>

<style scoped>
.session-footer {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
  margin-top: 12px;
  display: flex;
  gap: 22px;
  flex-wrap: wrap;
  font-size: 13px;
}
.session-footer.compact {
  padding: 6px 8px;
  margin-top: 6px;
  gap: 0;
  flex-direction: column;
  font-size: 11px;
  border: none;
  background: transparent;
}
</style>
