import type { MqttV3Config, MqttV5Config } from "$lib/gen"

export const DEFAULT_V5_CONFIG: MqttV5Config = {
    clean_start: true,
    keep_alive: undefined,
    session_expiration: undefined,
    receive_maximum: undefined,
    maximum_packet_size: undefined
}

export const DEFAULT_V3_CONFIG: MqttV3Config = {
    clean_session: true
}