<script setup>
import { reactive, watch } from 'vue';

const props = defineProps({ config: Object });
const emit = defineEmits(['save', 'close']);

const local = reactive(JSON.parse(JSON.stringify(props.config)));
watch(() => props.config, (v) => Object.assign(local, JSON.parse(JSON.stringify(v))), { deep: true });

function save() {
  emit('save', JSON.parse(JSON.stringify(local)));
  emit('close');
}
</script>

<template>
  <div class="panel">
    <div class="row" style="margin-bottom: 10px;">
      <strong>Settings</strong>
      <div class="spacer"></div>
      <button @click="$emit('close')">Close</button>
      <button class="primary" @click="save">Save</button>
    </div>

    <div style="display: grid; grid-template-columns: 180px 1fr; gap: 10px; align-items: center;">
      <label>Global hotkey</label>
      <input type="text" v-model="local.hotkey" placeholder="CmdOrCtrl+Shift+N" />

      <label>Start on system boot</label>
      <label class="row"><input type="checkbox" v-model="local.autostart" /> <span class="muted">Launch JackyNet at login</span></label>

      <label>Picture-in-Picture</label>
      <label class="row"><input type="checkbox" v-model="local.pip_enabled" /> <span class="muted">Show floating overlay</span></label>
    </div>

    <p class="muted" style="margin-top: 14px; font-size: 12px;">
      Rename interfaces or processes by double-clicking their name in the list. Aliases are stored in
      <span class="mono">~/.config/jackynet/config.json</span>.
    </p>
  </div>
</template>
