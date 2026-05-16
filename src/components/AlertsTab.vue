<script setup>
import { computed, reactive, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const props = defineProps({
  config: { type: Object, required: true },
  activeInterfaces: { type: Array, default: () => [] },
});
const emit = defineEmits(['changed']);

const COLORS = ['#fa5252', '#fd7e14', '#fcc419', '#51cf66', '#22b8cf', '#4dabf7', '#7950f2', '#e64980'];
const DURATIONS = [
  { value: '10s', label: '10 seconds' },
  { value: '1min', label: '1 minute' },
  { value: '5min', label: '5 minutes' },
  { value: 'continue', label: 'Continuous' },
];

const form = reactive({
  name: '',
  type: 'chronometer',
  hms: '00:30:00',
  iso: new Date(Date.now() + 60 * 60 * 1000).toISOString().slice(0, 16),
  bytes: 100,
  unit: 'MB',
  direction: 'combined',
  color: COLORS[0],
  notify_duration: '1min',
});

const error = ref('');

const upIfaces = computed(() => props.activeInterfaces.filter((i) => i.is_up));
const needsIfaceSelector = computed(() => upIfaces.value.length > 1);
const hasIface = computed(() => upIfaces.value.length > 0);
const selectedIface = computed({
  get: () => props.config.alert_iface || upIfaces.value[0]?.name || null,
  set: async (val) => {
    await invoke('set_alert_iface', { name: val });
    emit('changed');
  },
});

function ifaceLabel(name) {
  return props.config.interface_aliases?.[name] || name;
}

function bytesFromForm() {
  const factor = { B: 1, KB: 1024, MB: 1024 ** 2, GB: 1024 ** 3 }[form.unit] || 1;
  return Math.round(form.bytes * factor);
}

function makeId() {
  return `a-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
}

async function create() {
  error.value = '';
  if (!form.name.trim()) { error.value = 'Name is required'; return; }
  if (props.config.alerts?.length >= 10) { error.value = 'Maximum 10 alerts reached'; return; }

  let kind;
  if (form.type === 'chronometer') {
    if (!/^\d{1,3}:\d{1,2}:\d{1,2}$/.test(form.hms)) {
      error.value = 'Use HH:MM:SS format'; return;
    }
    kind = { type: 'chronometer', hms: form.hms };
  } else if (form.type === 'date') {
    if (!form.iso) { error.value = 'Pick a date'; return; }
    kind = { type: 'date', iso: form.iso };
  } else {
    const bytes = bytesFromForm();
    if (!bytes || bytes <= 0) { error.value = 'Threshold must be > 0'; return; }
    kind = { type: 'consumption', bytes, direction: form.direction };
  }

  const alert = {
    id: makeId(),
    name: form.name.trim(),
    color: form.color,
    notify_duration: form.notify_duration,
    paused: false,
    ...kind,
  };

  try {
    await invoke('add_alert', { alert });
    form.name = '';
    emit('changed');
  } catch (e) {
    error.value = String(e);
  }
}

async function removeAlert(id) {
  await invoke('remove_alert', { id });
  emit('changed');
}

async function togglePause(id) {
  await invoke('toggle_alert_pause', { id });
  emit('changed');
}

function summarizeAlert(a) {
  if (a.type === 'chronometer') return `Trigger at online ${a.hms}`;
  if (a.type === 'date') {
    const d = new Date(a.iso);
    return `Trigger on ${d.toLocaleString()}`;
  }
  if (a.type === 'consumption') {
    return `${a.direction} total ≥ ${fmtBytes(a.bytes)}`;
  }
  return '';
}

function fmtBytes(b) {
  if (!b) return '0 B';
  const u = ['B', 'KB', 'MB', 'GB', 'TB'];
  let i = 0; let v = Number(b);
  while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
  return `${v.toFixed(v < 10 ? 2 : 1)} ${u[i]}`;
}

function durationLabel(v) {
  return DURATIONS.find((d) => d.value === v)?.label || v;
}
</script>

<template>
  <div>
    <!-- No active interfaces -->
    <div v-if="!hasIface" class="panel muted" style="text-align:center;">
      An active network interface is required to create alerts.
    </div>

    <template v-else>
      <!-- Interface selector -->
      <div v-if="needsIfaceSelector" class="panel">
        <div class="row" style="gap: 10px; align-items: center;">
          <strong>Monitor interface:</strong>
          <select v-model="selectedIface">
            <option v-for="i in upIfaces" :key="i.name" :value="i.name">
              {{ ifaceLabel(i.name) }}{{ ifaceLabel(i.name) !== i.name ? ` (${i.name})` : '' }}
            </option>
          </select>
          <span class="muted" style="font-size: 12px;">applies to all alerts</span>
        </div>
      </div>

      <!-- Form -->
      <div class="panel">
        <h3 style="margin: 0 0 12px;">Create alert</h3>
        <div class="form-grid">
          <label>Name</label>
          <input type="text" v-model="form.name" placeholder="My alert" />

          <label>Type</label>
          <select v-model="form.type">
            <option value="chronometer">Chronometer (online time)</option>
            <option value="date">Date / time</option>
            <option value="consumption">Consumption</option>
          </select>

          <template v-if="form.type === 'chronometer'">
            <label>Online time</label>
            <input type="text" v-model="form.hms" placeholder="HH:MM:SS" />
          </template>

          <template v-else-if="form.type === 'date'">
            <label>Target date</label>
            <input type="datetime-local" v-model="form.iso" />
          </template>

          <template v-else>
            <label>Threshold</label>
            <div class="row" style="gap: 6px;">
              <input type="number" min="0" step="any" v-model.number="form.bytes" style="width: 100px;" />
              <select v-model="form.unit">
                <option>B</option><option>KB</option><option>MB</option><option>GB</option>
              </select>
              <select v-model="form.direction">
                <option value="download">Download</option>
                <option value="upload">Upload</option>
                <option value="combined">Combined</option>
              </select>
            </div>
          </template>

          <label>Color</label>
          <div class="color-row">
            <button
              v-for="c in COLORS"
              :key="c"
              type="button"
              class="color-swatch"
              :class="{ selected: form.color === c }"
              :style="{ background: c }"
              @click="form.color = c"
            ></button>
          </div>

          <label>Notify for</label>
          <select v-model="form.notify_duration">
            <option v-for="d in DURATIONS" :key="d.value" :value="d.value">{{ d.label }}</option>
          </select>
        </div>

        <div v-if="error" class="muted" style="color: var(--danger); margin-top: 10px;">{{ error }}</div>

        <div class="row" style="margin-top: 14px;">
          <div class="spacer"></div>
          <button class="primary" @click="create">Create alert</button>
        </div>
      </div>

      <!-- Alert list -->
      <div v-if="config.alerts?.length" class="panel">
        <h3 style="margin: 0 0 10px;">Alerts ({{ config.alerts.length }}/10)</h3>
        <div v-for="a in config.alerts" :key="a.id" class="alert-card">
          <div class="alert-color-bar" :style="{ background: a.color }"></div>
          <div class="alert-info">
            <div class="alert-name">
              <strong>{{ a.name }}</strong>
              <span v-if="a.paused" class="muted mono" style="font-size: 11px;">paused</span>
            </div>
            <div class="muted" style="font-size: 12px;">{{ summarizeAlert(a) }}</div>
            <div class="muted" style="font-size: 11px;">Notify: {{ durationLabel(a.notify_duration) }}</div>
          </div>
          <div class="row" style="gap: 6px;">
            <button @click="togglePause(a.id)">{{ a.paused ? 'Resume' : 'Pause' }}</button>
            <button class="danger" @click="removeAlert(a.id)">Delete</button>
          </div>
        </div>
      </div>
      <div v-else class="panel muted" style="text-align:center;">
        No alerts yet — create one above.
      </div>
    </template>
  </div>
</template>

<style scoped>
.form-grid {
  display: grid;
  grid-template-columns: 130px 1fr;
  gap: 10px 14px;
  align-items: center;
}
.color-row {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.color-swatch {
  width: 26px;
  height: 26px;
  border-radius: 5px;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
}
.color-swatch.selected {
  border-color: var(--text);
  box-shadow: 0 0 0 1px var(--panel);
}
.alert-card {
  display: flex;
  gap: 10px;
  padding: 10px;
  background: var(--panel-2);
  border-radius: 6px;
  margin-bottom: 8px;
  align-items: center;
}
.alert-card:last-child { margin-bottom: 0; }
.alert-color-bar {
  width: 4px;
  align-self: stretch;
  border-radius: 2px;
}
.alert-info { flex: 1; }
.alert-name { display: flex; gap: 8px; align-items: center; }
select, input[type=number], input[type=datetime-local] {
  background: var(--panel-2);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 5px 8px;
  border-radius: 4px;
  font-size: 13px;
  font-family: inherit;
}
</style>
