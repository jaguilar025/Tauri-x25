<script setup>
import { ref } from 'vue';

const props = defineProps({
  processes: Array,
  aliases: Object,
  displayMode: { type: String, default: 'auto' },
});
const emit = defineEmits(['kill', 'rename']);

const expanded = ref(new Set());
const editing = ref(null);
const draft = ref('');

function rowKey(p) { return `${p.pid}:${p.identity}`; }
function aliasKey(p) { return p.identity || p.program; }

function toggle(p) {
  const k = rowKey(p);
  if (expanded.value.has(k)) expanded.value.delete(k);
  else expanded.value.add(k);
  expanded.value = new Set(expanded.value);
}

function startEdit(key) {
  editing.value = key;
  draft.value = props.aliases?.[key] || '';
}
function commitEdit(key) {
  emit('rename', key, draft.value);
  editing.value = null;
}
function display(p) {
  return props.aliases?.[aliasKey(p)] || shortPath(aliasKey(p));
}
function shortPath(path) {
  if (!path) return '?';
  const parts = path.split('/').filter(Boolean);
  if (parts.length <= 2) return path;
  return `…/${parts.slice(-2).join('/')}`;
}
function fmt(kb) {
  if (kb == null) return '–';
  const bps = kb * 1024;
  switch (props.displayMode) {
    case 'B/s':  return `${bps.toFixed(0)} B/s`;
    case 'KB/s': return `${kb.toFixed(2)} KB/s`;
    case 'MB/s': return `${(kb / 1024).toFixed(3)} MB/s`;
    case 'GB/s': return `${(kb / 1024 / 1024).toFixed(4)} GB/s`;
    default:
      if (kb < 1) return `${bps.toFixed(0)} B/s`;
      if (kb < 1024) return `${kb.toFixed(1)} KB/s`;
      if (kb < 1024 * 1024) return `${(kb / 1024).toFixed(2)} MB/s`;
      return `${(kb / 1024 / 1024).toFixed(3)} GB/s`;
  }
}
</script>

<template>
  <div class="panel">
    <table>
      <thead>
        <tr>
          <th></th>
          <th>Program</th>
          <th>PID</th>
          <th>User</th>
          <th>Download</th>
          <th>Upload</th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="!processes.length">
          <td colspan="6" class="muted" style="text-align:center; padding: 20px;">
            No active network processes. nethogs requires sudo — check permissions.
          </td>
        </tr>
        <template v-for="p in processes" :key="rowKey(p)">
          <tr class="clickable-row" @click="toggle(p)">
            <td class="muted" style="width:18px;">{{ expanded.has(rowKey(p)) ? '▾' : '▸' }}</td>
            <td :title="p.program">{{ display(p) }}</td>
            <td class="mono">{{ p.pid }}</td>
            <td class="muted">{{ p.user || '–' }}</td>
            <td class="mono">{{ fmt(p.rx_kbs) }}</td>
            <td class="mono">{{ fmt(p.tx_kbs) }}</td>
          </tr>
          <tr v-if="expanded.has(rowKey(p))" class="detail-row">
            <td></td>
            <td colspan="5">
              <div class="detail-grid">
                <div class="muted">Program</div>
                <div class="mono selectable">{{ p.program }}</div>
                <div v-if="p.identity && p.identity !== p.program" class="muted">Resolved binary</div>
                <div v-if="p.identity && p.identity !== p.program" class="mono selectable">{{ p.identity }}</div>
                <div class="muted">Cmdline</div>
                <div class="mono selectable">{{ p.cmdline || '—' }}</div>
                <div class="muted">Alias</div>
                <div>
                  <template v-if="editing === aliasKey(p)">
                    <input
                      type="text"
                      v-model="draft"
                      :placeholder="shortPath(aliasKey(p))"
                      @keyup.enter="commitEdit(aliasKey(p))"
                      @keyup.escape="editing = null"
                      @blur="commitEdit(aliasKey(p))"
                      autofocus
                    />
                  </template>
                  <template v-else>
                    <span>{{ aliases?.[aliasKey(p)] || '—' }}</span>
                  </template>
                </div>
              </div>
              <div class="row" style="margin-top: 10px;">
                <button @click.stop="startEdit(aliasKey(p))">
                  {{ aliases?.[aliasKey(p)] ? 'Edit alias' : 'Rename' }}
                </button>
                <button
                  v-if="aliases?.[aliasKey(p)]"
                  @click.stop="$emit('rename', aliasKey(p), '')"
                >Clear alias</button>
                <div class="spacer"></div>
                <button class="danger" :disabled="p.pid <= 1" @click.stop="$emit('kill', p.pid)">
                  Kill PID {{ p.pid }}
                </button>
              </div>
            </td>
          </tr>
        </template>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.clickable-row { cursor: pointer; }
.clickable-row:hover { background: var(--panel-2); }
.detail-row td { background: var(--panel-2); padding: 12px 14px; }
.detail-grid {
  display: grid;
  grid-template-columns: 110px 1fr;
  gap: 6px 12px;
  font-size: 13px;
}
.selectable { user-select: text; word-break: break-all; }
</style>
