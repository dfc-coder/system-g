export type Availability = 'online' | 'offline'
export type TankStatus = 'unknown' | 'water_available' | 'water_low' | 'sensor_fault'
export type IrrigationStatus =
  | 'idle'
  | 'checking_tank'
  | 'pumping'
  | 'absorbing'
  | 'complete'
  | 'blocked'

export interface TelemetrySnapshot {
  schema_version: number
  device_id: string
  sequence: number
  observed_at_unix_ms: number | null
  uptime_seconds: number
  clock_ok: boolean
  lights_on: boolean
  fans_on: boolean
  pump_on: boolean
  temperature_c: number | null
  humidity_pct: number | null
  soil_moisture_pct: number | null
  tank: TankStatus
  irrigation: IrrigationStatus
  fault: string | null
}

export interface CommandAcknowledgement {
  schema_version: number
  device_id: string
  command_id: string
  accepted: boolean
  reason: string | null
  acknowledged_at_unix_ms: number | null
}

export interface DeviceRecord {
  device_id: string
  availability: Availability
  telemetry: TelemetrySnapshot | null
  latest_acknowledgement: CommandAcknowledgement | null
  latest_event: unknown
  last_seen_unix_ms: number
}

export type DeviceCommand =
  | {
      type: 'set_override'
      target: 'lights' | 'fans'
      mode: 'auto' | 'on' | 'off'
      duration_seconds: number | null
    }
  | { type: 'start_irrigation_cycle' }
  | { type: 'cancel_irrigation' }
  | { type: 'request_snapshot' }

export interface CommandEnvelope {
  schema_version: number
  command_id: string
  issued_at_unix_ms: number
  command: DeviceCommand
}
