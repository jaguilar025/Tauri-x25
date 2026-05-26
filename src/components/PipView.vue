<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import SessionFooter from './SessionFooter.vue';
import AlertIndicators from './AlertIndicators.vue';

const processes = ref([]);
const aliases = ref({});
const ifaceAliases = ref({});
let unlisten = null;

function fmt(kb) {
  if (kb == null) return '–';
  return kb.toFixed(1);
}
function shortPath(path) {
  if (!path) return '?';
  const parts = path.split('/').filter(Boolean);
  if (parts.length <= 1) return path;
  return parts[parts.length - 1];
}
function aliasKey(p) { return p.identity || p.program; }
function display(p) { return aliases.value[aliasKey(p)] || shortPath(aliasKey(p)); }

async function close() { await invoke('toggle_pip'); }

async function startResize(direction, e) {
  e.preventDefault();
  e.stopPropagation();
  try { await getCurrentWindow().startResizeDragging(direction); } catch (_) {}
}

onMounted(async () => {
  const cfg = await invoke('get_config');
  aliases.value = cfg.process_aliases || {};
  ifaceAliases.value = cfg.interface_aliases || {};
  processes.value = await invoke('nethogs_snapshot').catch(() => []);
  unlisten = await listen('nethogs:update', (e) => { processes.value = e.payload; });
});
onUnmounted(() => { if (unlisten) unlisten(); });
</script>

<template>
  <div class="pip-window" data-tauri-drag-region>
    <div class="row pip-header" data-tauri-drag-region>
      <img src="/icon.png" alt="" class="pip-logo" data-tauri-drag-region />
      <strong style="font-size: 12px;" data-tauri-drag-region>Tauri x25</strong>
      <AlertIndicators />
      <div class="spacer" data-tauri-drag-region></div>
      <button @click="close" title="Exit PiP">✕</button>
    </div>
    <table>
      <tbody>
        <tr v-if="!processes.length"><td class="muted" colspan="3">No data</td></tr>
        <tr v-for="p in processes.slice(0, 6)" :key="p.pid + ':' + (p.identity || p.program)">
          <td class="trunc" :title="p.program">{{ display(p) }}</td>
          <td class="mono small">↓{{ fmt(p.rx_kbs) }}</td>
          <td class="mono small">↑{{ fmt(p.tx_kbs) }}</td>
        </tr>
      </tbody>
    </table>
    <div v-if="processes.length" class="unit-legend muted">values in KB/s</div>
    <SessionFooter compact :aliases="ifaceAliases" />

    <!-- Resize handles -->
    <div class="rz rz-n"  @mousedown="startResize('North', $event)"></div>
    <div class="rz rz-s"  @mousedown="startResize('South', $event)"></div>
    <div class="rz rz-e"  @mousedown="startResize('East', $event)"></div>
    <div class="rz rz-w"  @mousedown="startResize('West', $event)"></div>
    <div class="rz rz-ne" @mousedown="startResize('NorthEast', $event)"></div>
    <div class="rz rz-nw" @mousedown="startResize('NorthWest', $event)"></div>
    <div class="rz rz-se" @mousedown="startResize('SouthEast', $event)"></div>
    <div class="rz rz-sw" @mousedown="startResize('SouthWest', $event)"></div>
  </div>
</template>

<style scoped>
.pip-window {
  padding: 8px;
  font-size: 12px;
  user-select: none;
  cursor: move;
  height: 100vh;
  box-sizing: border-box;
  position: relative;
}
.pip-header { margin-bottom: 6px; gap: 6px; }
.pip-header button { cursor: pointer; }
.pip-logo {
  width: 16px;
  height: 16px;
  object-fit: contain;
  border-radius: 3px;
}
.trunc {
  max-width: 130px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.small { font-size: 11px; }
.unit-legend {
  font-size: 10px;
  text-align: right;
  padding: 2px 4px 4px;
  font-style: italic;
}

/* Resize handles: invisible bands on edges + corners */
.rz { position: fixed; z-index: 9999; }
.rz-n  { top: 0; left: 6px; right: 6px; height: 5px; cursor: n-resize; }
.rz-s  { bottom: 0; left: 6px; right: 6px; height: 5px; cursor: s-resize; }
.rz-e  { top: 6px; bottom: 6px; right: 0; width: 5px; cursor: e-resize; }
.rz-w  { top: 6px; bottom: 6px; left: 0; width: 5px; cursor: w-resize; }
.rz-ne { top: 0; right: 0; width: 10px; height: 10px; cursor: ne-resize; }
.rz-nw { top: 0; left: 0; width: 10px; height: 10px; cursor: nw-resize; }
.rz-se { bottom: 0; right: 0; width: 12px; height: 12px; cursor: se-resize; }
.rz-sw { bottom: 0; left: 0; width: 10px; height: 10px; cursor: sw-resize; }

/* Subtle visual cue for SE corner */
.rz-se::after {
  content: '';
  position: absolute;
  bottom: 2px; right: 2px;
  width: 6px; height: 6px;
  border-right: 2px solid var(--muted);
  border-bottom: 2px solid var(--muted);
  opacity: 0.5;
}
</style>
