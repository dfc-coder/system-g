<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { fetchDevices, openEventStream, publishCommand } from './api'
import type { DeviceCommand, DeviceRecord } from './types'

const devices = ref<DeviceRecord[]>([])
const selectedDeviceId = ref<string | null>(null)
const loading = ref(true)
const sending = ref(false)
const connected = ref(false)
const errorMessage = ref<string | null>(null)
let eventSource: EventSource | null = null
let refreshTimer: number | null = null

const selectedDevice = computed(() => {
  const selected = devices.value.find((device) => device.device_id === selectedDeviceId.value)
  return selected ?? devices.value[0] ?? null
})

async function refresh(): Promise<void> {
  try {
    devices.value = await fetchDevices()
    if (!selectedDeviceId.value && devices.value.length > 0) {
      selectedDeviceId.value = devices.value[0].device_id
    }
    errorMessage.value = null
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : 'Unable to load devices'
  } finally {
    loading.value = false
  }
}

function scheduleRefresh(): void {
  if (refreshTimer !== null) {
    window.clearTimeout(refreshTimer)
  }
  refreshTimer = window.setTimeout(() => void refresh(), 100)
}

async function send(command: DeviceCommand): Promise<void> {
  const device = selectedDevice.value
  if (!device || sending.value) return

  sending.value = true
  try {
    await publishCommand(device.device_id, command)
    errorMessage.value = null
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : 'Command failed'
  } finally {
    sending.value = false
  }
}

function override(target: 'lights' | 'fans', mode: 'auto' | 'on' | 'off'): void {
  void send({
    type: 'set_override',
    target,
    mode,
    duration_seconds: mode === 'auto' ? null : 1800,
  })
}

function formatMetric(value: number | null | undefined, suffix: string): string {
  return value === null || value === undefined ? '—' : `${value.toFixed(1)}${suffix}`
}

function formatRelative(timestamp: number): string {
  const seconds = Math.max(0, Math.round((Date.now() - timestamp) / 1000))
  if (seconds < 60) return `${seconds}s ago`
  const minutes = Math.round(seconds / 60)
  if (minutes < 60) return `${minutes}m ago`
  return `${Math.round(minutes / 60)}h ago`
}

onMounted(() => {
  void refresh()
  eventSource = openEventStream(
    () => {
      connected.value = true
      scheduleRefresh()
    },
    () => {
      connected.value = false
    },
  )
})

onUnmounted(() => {
  eventSource?.close()
  if (refreshTimer !== null) window.clearTimeout(refreshTimer)
})
</script>

<template>
  <main class="shell">
    <header class="topbar">
      <div>
        <p class="eyebrow">Indoor environment control</p>
        <h1>Growntrol</h1>
      </div>
      <div class="connection" :class="{ online: connected }">
        <span class="connection-dot" />
        {{ connected ? 'Live MQTT feed' : 'Waiting for events' }}
      </div>
    </header>

    <section v-if="errorMessage" class="error-banner">{{ errorMessage }}</section>

    <section v-if="loading" class="empty-state">Loading devices…</section>
    <section v-else-if="devices.length === 0" class="empty-state">
      No devices have published telemetry yet.
    </section>

    <template v-else>
      <nav class="device-tabs" aria-label="Devices">
        <button
          v-for="device in devices"
          :key="device.device_id"
          class="device-tab"
          :class="{ active: selectedDevice?.device_id === device.device_id }"
          @click="selectedDeviceId = device.device_id"
        >
          <span>{{ device.device_id }}</span>
          <span class="status-chip" :class="device.availability">{{ device.availability }}</span>
        </button>
      </nav>

      <section v-if="selectedDevice" class="dashboard-grid">
        <article class="hero-card panel">
          <div>
            <p class="label">Device</p>
            <h2>{{ selectedDevice.device_id }}</h2>
            <p class="muted">Last update {{ formatRelative(selectedDevice.last_seen_unix_ms) }}</p>
          </div>
          <div class="system-state">
            <span class="state-orb" :class="selectedDevice.availability" />
            <div>
              <strong>{{ selectedDevice.telemetry?.fault ?? 'System nominal' }}</strong>
              <span>{{ selectedDevice.telemetry?.irrigation ?? 'No telemetry' }}</span>
            </div>
          </div>
        </article>

        <article class="metric panel">
          <span class="label">Temperature</span>
          <strong>{{ formatMetric(selectedDevice.telemetry?.temperature_c, '°C') }}</strong>
        </article>
        <article class="metric panel">
          <span class="label">Humidity</span>
          <strong>{{ formatMetric(selectedDevice.telemetry?.humidity_pct, '%') }}</strong>
        </article>
        <article class="metric panel">
          <span class="label">Soil moisture</span>
          <strong>{{ formatMetric(selectedDevice.telemetry?.soil_moisture_pct, '%') }}</strong>
        </article>
        <article class="metric panel">
          <span class="label">Tank</span>
          <strong class="text-value">{{ selectedDevice.telemetry?.tank ?? 'unknown' }}</strong>
        </article>

        <article class="panel controls-card">
          <div class="section-heading">
            <div>
              <p class="label">Manual overrides</p>
              <h3>Lights and ventilation</h3>
            </div>
            <span class="muted">Overrides expire after 30 minutes</span>
          </div>

          <div class="control-row">
            <span>Lights</span>
            <div class="button-group">
              <button :disabled="sending" @click="override('lights', 'auto')">Auto</button>
              <button :disabled="sending" @click="override('lights', 'on')">On</button>
              <button :disabled="sending" @click="override('lights', 'off')">Off</button>
            </div>
          </div>
          <div class="control-row">
            <span>Fans</span>
            <div class="button-group">
              <button :disabled="sending" @click="override('fans', 'auto')">Auto</button>
              <button :disabled="sending" @click="override('fans', 'on')">On</button>
              <button :disabled="sending" @click="override('fans', 'off')">Off</button>
            </div>
          </div>
        </article>

        <article class="panel controls-card irrigation-card">
          <div class="section-heading">
            <div>
              <p class="label">Irrigation</p>
              <h3>Safe cycle control</h3>
            </div>
            <span class="muted">Firmware safety rules remain authoritative</span>
          </div>
          <div class="irrigation-actions">
            <button
              class="primary"
              :disabled="sending"
              @click="send({ type: 'start_irrigation_cycle' })"
            >
              Start eligible cycle
            </button>
            <button :disabled="sending" @click="send({ type: 'cancel_irrigation' })">
              Cancel cycle
            </button>
            <button :disabled="sending" @click="send({ type: 'request_snapshot' })">
              Request snapshot
            </button>
          </div>
        </article>
      </section>
    </template>
  </main>
</template>
