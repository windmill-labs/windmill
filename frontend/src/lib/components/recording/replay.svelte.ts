import type { ActiveReplayData } from './types'

// Module-level so every JobLoader in the tree sees the same replay, wherever it
// mounts (portals/drawers included).
let activeReplay: ActiveReplayData | undefined = $state<ActiveReplayData | undefined>(undefined)
let replayStartTime: number = 0

export function getActiveReplay() {
	return activeReplay
}

export function setActiveReplay(r: ActiveReplayData | undefined) {
	activeReplay = r
	replayStartTime = r ? Date.now() : 0
}

/** The wall-clock moment the current replay started; event `t` values are
 * scheduled relative to it so late-discovered sub-jobs stay in sync. */
export function getReplayStartTime() {
	return replayStartTime
}
