import { writable } from 'svelte/store'
import type { GetInstanceUiResponse } from '$lib/gen'

/**
 * The last `/settings/instance_ui` response: the announcement banner and accent color.
 * `undefined` until one lands, and left as is when a fetch fails, so a transient error
 * never retracts an announcement still in force. Written only by `InstanceUiSync`.
 */
export const instanceUi = writable<GetInstanceUiResponse | undefined>(undefined)
