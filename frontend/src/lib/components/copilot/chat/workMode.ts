// A leaf: the chat hosts and the session UI around them read these without loading the manager.

/** What a session chat may do, picked in the composer beside the confirmations posture. */
export enum AIWorkMode {
	/** Reads and runs deployed items, and changes nothing. Locks the session's preview too. */
	USE = 'use',
	/** Researches, then proposes a plan for the user's approval. Runs nothing. */
	PLAN = 'plan',
	BUILD = 'build'
}

/** Why a session cannot leave Use, and where the user can build instead. */
export type WorkModeLock = {
	reason: string
	escape?: { label: string; action: () => void }
}
