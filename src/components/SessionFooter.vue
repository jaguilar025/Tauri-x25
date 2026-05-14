<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const props = defineProps({
  compact: { type: Boolean, default: false },
  aliases: { type: Object, default: () => ({}) },
});

const interfaces = ref([]); // Array<IfaceSession>
const now = ref(Date.now());
const summed = ref(new Set()); // iface names showing combined ↑+↓
const dateFormatIdx = ref({}); // iface name → 0..5
const onlineFormatIdx = ref({}); // iface name → 0..1
let timer = null;
let unlisten = null;

// Defaults differ per view: PiP starts summed totals + time-only date format;
// Dashboard starts with separated totals + full date format.
const defaultDateFmt = () => (props.compact ? 2 : 0);
const defaultSummed = () => props.compact;
const defaultOnlineFmt = () => 0;

function isSummed(name) {
  const flipped = summed.value.has(name);
  return defaultSummed() ? !flipped : flipped;
}

function toggleSummed(name) {
  if (summed.value.has(name)) summed.value.delete(name);
  else summed.value.add(name);
  summed.value = new Set(summed.value);
}

function dateFmtFor(name) {
  return dateFormatIdx.value[name] ?? defaultDateFmt();
}
function cycleDateFormat(name) {
  const i = dateFmtFor(name);
  dateFormatIdx.value = { ...dateFormatIdx.value, [name]: (i + 1) % 6 };
}

function onlineFmtFor(name) {
  return onlineFormatIdx.value[name] ?? defaultOnlineFmt();
}
function cycleOnlineFormat(name) {
  const i = onlineFmtFor(name);
  onlineFormatIdx.value = { ...onlineFormatIdx.value, [name]: (i + 1) % 2 };
}

async function resetIface(name) {
  try {
    interfaces.value = await invoke('reset_iface_session', { name });
  } catch (_) {}
}

function fmtBytes(b) {
  if (!b) return '0 B';
  const u = ['B','KB','MB','GB','TB'];
  let i = 0; let v = Number(b);
  while (v >= 1024 && i < u.length-1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 ? 2 : v < 100 ? 1 : 0)} ${u[i]}`;
}

const MONTHS = ['January','February','March','April','May','June','July','August','September','October','November','December'];

function sessionLabel(unixMs, mode = 0) {
  if (!unixMs) return '—';
  const d = new Date(unixMs);
  const day = d.getDate();
  const monthName = MONTHS[d.getMonth()];
  const monthNum = (d.getMonth() + 1).toString().padStart(2, '0');
  const year = d.getFullYear();
  const yearShort = year.toString().slice(-2);
  const dayPad = day.toString().padStart(2, '0');
  const h24 = d.getHours();
  const m = d.getMinutes().toString().padStart(2, '0');
  const ampm = h24 >= 12 ? 'pm' : 'am';
  const h12 = (h24 % 12 || 12).toString().padStart(2, '0');
  const h24p = h24.toString().padStart(2, '0');

  const longDate = `${day}/${monthName}/${year}`;
  const shortDate = `${dayPad}/${monthNum}/${yearShort}`;
  const time12 = `${h12}:${m}${ampm}`;
  const time24 = `${h24p}:${m}`;

  switch (mode) {
    case 0: return `${longDate} ${time12}`;
    case 1: return `${longDate} ${time24}`;
    case 2: return time12;
    case 3: return time24;
    case 4: return `${shortDate} ${time12}`;
    case 5: return `${shortDate} ${time24}`;
    default: return `${longDate} ${time12}`;
  }
}

function onlineLabel(unixMs, mode = 0) {
  if (!unixMs) return mode === 1 ? '0 seg' : '0:00:00';
  const secs = Math.max(0, Math.floor((now.value - unixMs) / 1000));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (mode === 1) {
    const parts = [];
    if (h > 0) parts.push(`${h} hrs`);
    if (h > 0 || m > 0) parts.push(`${m} min`);
    parts.push(`${s} seg`);
    return parts.join(' ');
  }
  return `${h}:${m.toString().padStart(2,'0')}:${s.toString().padStart(2,'0')}`;
}

function displayName(name) {
  return props.aliases?.[name] || name;
}

const activeInterfaces = computed(() =>
  interfaces.value.filter((i) => i.is_up)
);

onMounted(async () => {
  try { interfaces.value = await invoke('get_session_stats'); } catch (_) {}
  unlisten = await listen('session:update', (e) => { interfaces.value = e.payload || []; });
  timer = setInterval(() => { now.value = Date.now(); }, 1000);
});
onUnmounted(() => {
  if (timer) clearInterval(timer);
  if (unlisten) unlisten();
});
</script>

<template>
  <div :class="['session-wrap', { compact }]">
    <div v-if="!activeInterfaces.length" :class="['session-block', { compact }]">
      <div class="muted">No active interfaces detected</div>
    </div>
    <div
      v-for="iface in activeInterfaces"
      :key="iface.name"
      :class="['session-block', { compact }]"
    >
      <div class="iface-title">
        <span class="status-dot online"></span>
        <span class="iface-name">{{ displayName(iface.name) }}</span>
        <span v-if="!compact && aliases?.[iface.name]" class="muted mono iface-real">({{ iface.name }})</span>
        <div class="spacer"></div>
        <button v-if="!compact" class="reset-btn" @click="resetIface(iface.name)" title="Reset session">⟳</button>
      </div>
      <div class="active-row" @click="cycleDateFormat(iface.name)" title="Click to change format">
        <span class="muted">Active session:</span>
        {{ sessionLabel(iface.started_unix_ms, dateFmtFor(iface.name)) }}
      </div>
      <div class="acc-row" @click="toggleSummed(iface.name)" :title="isSummed(iface.name) ? 'Show separate ↑/↓' : 'Show combined total'">
        <span class="muted">Accumulated total:</span>
        <template v-if="isSummed(iface.name)">
          {{ fmtBytes(iface.total_tx_bytes + iface.total_rx_bytes) }}
        </template>
        <template v-else>
          ↑{{ fmtBytes(iface.total_tx_bytes) }}
          ↓{{ fmtBytes(iface.total_rx_bytes) }}
        </template>
      </div>
      <div class="online-row" @click="cycleOnlineFormat(iface.name)" title="Click to change format">
        <span class="muted">Online:</span>
        <span class="mono">{{ onlineLabel(iface.started_unix_ms, onlineFmtFor(iface.name)) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.session-wrap {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 12px;
}
.session-wrap.compact {
  gap: 6px;
  margin-top: 6px;
}
.session-block {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
  font-size: 13px;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.session-block.compact {
  padding: 6px 8px;
  font-size: 11px;
  border: none;
  background: transparent;
  gap: 1px;
}
.iface-title {
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 6px;
}
.spacer { flex: 1; }
.reset-btn {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--muted);
  padding: 2px 8px;
  font-size: 13px;
  line-height: 1;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 400;
}
.reset-btn:hover {
  background: var(--panel-2);
  color: var(--text);
  border-color: var(--accent);
}
.acc-row,
.active-row,
.online-row {
  cursor: pointer;
  user-select: none;
  display: flex;
  gap: 6px;
  align-items: baseline;
  flex-wrap: wrap;
}
.active-row:hover,
.online-row:hover { color: var(--accent); }
.acc-row:hover {
  color: var(--accent);
}
.session-block.compact .reset-btn {
  padding: 0 4px;
  font-size: 11px;
}
.iface-real {
  font-weight: 400;
  font-size: 11px;
}
.status-dot {
  display: inline-block;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--muted);
}
.status-dot.online {
  background: var(--ok);
  box-shadow: 0 0 6px rgba(81, 207, 102, 0.6);
}
</style>
