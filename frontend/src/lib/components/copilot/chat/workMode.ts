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

/** The answer to a call approved after its session switched to Use: the card or form it
 * waited on was approved, but the mode the session is in now withholds the tool. */
export const USE_MODE_WITHDRAWN = {
	label: 'Not run: the session is in Use',
	result:
		'Not run: the user switched this session to Use mode, which changes nothing, while this call waited for them. Do not retry it; it can run once they switch back to Build.'
}
