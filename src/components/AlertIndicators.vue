<script setup>
import { onMounted, onUnmounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const active = ref([]);
let unlisten = null;

async function dismiss(id) {
  try {
    await invoke('dismiss_alert', { id });
    active.value = active.value.filter((a) => a.id !== id);
  } catch (_) {}
}

onMounted(async () => {
  unlisten = await listen('alerts:active', (e) => { active.value = e.payload || []; });
});
onUnmounted(() => { if (unlisten) unlisten(); });
</script>

<template>
  <div v-if="active.length" class="alert-indicators">
    <button
      v-for="a in active"
      :key="a.id"
      class="alert-dot"
      :style="{ '--alert-color': a.color }"
      :title="`${a.name}`"
      @click="dismiss(a.id)"
    ></button>
  </div>
</template>

<style scoped>
.alert-indicators {
  display: inline-flex;
  gap: 4px;
  align-items: center;
}
.alert-dot {
  position: relative;
  width: 16px;
  height: 16px;
  border: 1px solid rgba(255,255,255,0.25);
  background: var(--alert-color);
  border-radius: 3px;
  cursor: pointer;
  padding: 0;
  animation: alertBlink 1s steps(2, start) infinite;
  box-shadow: 0 0 8px var(--alert-color);
  transition: transform 0.12s ease;
}
.alert-dot::after {
  content: '✕';
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  color: #fff;
  text-shadow: 0 0 2px rgba(0,0,0,0.7);
  opacity: 0;
  transition: opacity 0.15s ease;
  pointer-events: none;
}
.alert-dot:hover {
  transform: scale(1.2);
  animation-play-state: paused;
  opacity: 1 !important;
}
.alert-dot:hover::after { opacity: 1; }
@keyframes alertBlink {
  0%, 49% { opacity: 1; }
  50%, 100% { opacity: 0.25; }
}
</style>
