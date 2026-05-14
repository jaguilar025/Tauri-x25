<script setup>
import { onMounted, onUnmounted, ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import Dashboard from './components/Dashboard.vue';
import PipView from './components/PipView.vue';

const isPip = ref(false);
const route = new URLSearchParams(window.location.search).get('view');

onMounted(async () => {
  if (route === 'pip') {
    isPip.value = true;
    document.body.classList.add('pip');
  }
});
</script>

<template>
  <PipView v-if="isPip" />
  <Dashboard v-else />
</template>
