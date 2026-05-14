<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const props = defineProps({
  interfaces: Array,
  aliases: Object,
});
const emit = defineEmits(['rename']);

const expanded = ref(new Set());
const editing = ref(null);
const draft = ref('');
const liveStatus = ref({}); // name → is_up
let unlistenStatus = null;

onMounted(async () => {
  try {
    const stats = await invoke('get_session_stats');
    const map = {};
    for (const s of stats) map[s.name] = s.is_up;
    liveStatus.value = map;
  } catch (_) {}
  unlistenStatus = await listen('session:update', (e) => {
    const map = {};
    for (const s of (e.payload || [])) map[s.name] = s.is_up;
    liveStatus.value = map;
  });
});

onUnmounted(() => { if (unlistenStatus) unlistenStatus(); });

function isOnline(name) { return !!liveStatus.value[name]; }

function toggle(name) {
  if (expanded.value.has(name)) expanded.value.delete(name);
  else expanded.value.add(name);
  expanded.value = new Set(expanded.value);
}
function startEdit(iface) {
  editing.value = iface;
  draft.value = props.aliases?.[iface] || '';
}
function commitEdit(iface) {
  emit('rename', iface, draft.value);
  editing.value = null;
}
function clearAlias(iface) { emit('rename', iface, ''); }
function display(name) { return props.aliases?.[name] || name; }
function fmtBytes(b) {
  if (b == null) return '–';
  const u = ['B','KB','MB','GB','TB'];
  let i = 0; let v = Number(b);
  while (v >= 1024 && i < u.length-1) { v /= 1024; i++; }
  return `${v.toFixed(2)} ${u[i]}`;
}
</script>

<template>
  <div>
    <div v-if="!interfaces.length" class="panel muted" style="text-align:center;">
      No vnstat data. Run <span class="mono">vnstat -u -i &lt;iface&gt;</span> or wait for the daemon to collect samples.
    </div>

    <div v-for="iface in interfaces" :key="iface.name" class="panel iface-card">
      <div class="row iface-header" @click="toggle(iface.name)">
        <span class="muted" style="width:18px;">{{ expanded.has(iface.name) ? '▾' : '▸' }}</span>
        <span
          class="status-dot"
          :class="{ online: isOnline(iface.name), offline: !isOnline(iface.name) }"
          :title="isOnline(iface.name) ? 'Active' : 'Inactive'"
        ></span>
        <strong>{{ display(iface.name) }}</strong>
        <div class="spacer"></div>
        <span class="muted mono">total ↓ {{ fmtBytes(iface.total_rx) }} ↑ {{ fmtBytes(iface.total_tx) }}</span>
      </div>

      <div v-if="expanded.has(iface.name)" class="iface-body">
        <div class="detail-grid" style="margin-bottom: 12px;">
          <div class="muted">Interface</div>
          <div class="mono selectable">{{ iface.name }}</div>
          <div class="muted">Alias</div>
          <div>
            <template v-if="editing === iface.name">
              <input
                type="text"
                v-model="draft"
                :placeholder="iface.name"
                @keyup.enter="commitEdit(iface.name)"
                @keyup.escape="editing = null"
                @blur="commitEdit(iface.name)"
                autofocus
              />
            </template>
            <template v-else>
              <span>{{ aliases?.[iface.name] || '—' }}</span>
            </template>
          </div>
        </div>

        <div class="row" style="margin-bottom: 14px;">
          <button @click.stop="startEdit(iface.name)">
            {{ aliases?.[iface.name] ? 'Edit alias' : 'Rename' }}
          </button>
          <button v-if="aliases?.[iface.name]" @click.stop="clearAlias(iface.name)">
            Clear alias
          </button>
        </div>

        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px;">
          <div>
            <h4 style="margin:0 0 6px;" class="muted">Daily</h4>
            <table>
              <thead><tr><th>Date</th><th>Down</th><th>Up</th></tr></thead>
              <tbody>
                <tr v-for="d in iface.daily?.slice(0, 10)" :key="d.date">
                  <td class="mono">{{ d.date }}</td>
                  <td class="mono">{{ fmtBytes(d.rx) }}</td>
                  <td class="mono">{{ fmtBytes(d.tx) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div>
            <h4 style="margin:0 0 6px;" class="muted">Monthly</h4>
            <table>
              <thead><tr><th>Month</th><th>Down</th><th>Up</th></tr></thead>
              <tbody>
                <tr v-for="m in iface.monthly?.slice(0, 12)" :key="m.date">
                  <td class="mono">{{ m.date }}</td>
                  <td class="mono">{{ fmtBytes(m.rx) }}</td>
                  <td class="mono">{{ fmtBytes(m.tx) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.iface-card { padding: 0; overflow: hidden; }
.iface-header {
  padding: 12px;
  cursor: pointer;
  gap: 6px;
}
.iface-header:hover { background: var(--panel-2); }
.iface-body {
  padding: 14px;
  border-top: 1px solid var(--border);
  background: var(--panel-2);
}
.detail-grid {
  display: grid;
  grid-template-columns: 110px 1fr;
  gap: 6px 12px;
  font-size: 13px;
}
.selectable { user-select: text; word-break: break-all; }
.status-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.online {
  background: var(--ok);
  box-shadow: 0 0 6px rgba(81, 207, 102, 0.6);
}
.status-dot.offline {
  background: var(--danger);
  opacity: 0.55;
}
</style>
