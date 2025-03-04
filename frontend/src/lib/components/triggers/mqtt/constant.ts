import type { MqttV3Config, MqttV5Config } from "$lib/gen"

export const DEFAULT_V5_CONFIG: MqttV5Config = {
    clean_start: true,
}

export const DEFAULT_V3_CONFIG: MqttV3Config = {
    clean_session: true
}