import type { CommandEnvelope, DeviceCommand, DeviceRecord } from './types'

const API_BASE = import.meta.env.VITE_API_BASE ?? ''

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      ...init?.headers,
    },
    ...init,
  })

  if (!response.ok) {
    const detail = await response.text()
    throw new Error(detail || `Request failed with ${response.status}`)
  }

  return response.json() as Promise<T>
}

export function fetchDevices(): Promise<DeviceRecord[]> {
  return request<DeviceRecord[]>('/api/devices')
}

export function publishCommand(
  deviceId: string,
  command: DeviceCommand,
): Promise<CommandEnvelope> {
  return request<CommandEnvelope>(`/api/devices/${encodeURIComponent(deviceId)}/commands`, {
    method: 'POST',
    body: JSON.stringify(command),
  })
}

export function openEventStream(onEvent: () => void, onError: () => void): EventSource {
  const source = new EventSource(`${API_BASE}/api/events`)
  source.onmessage = onEvent
  source.onerror = onError
  return source
}
