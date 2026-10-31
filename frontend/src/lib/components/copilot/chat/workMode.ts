// A leaf: the chat hosts and the session UI around them read these without loading the manager.

/** What a session chat may do, picked in the composer beside the confirmations posture. */
export enum AIWorkMode {
	/** Reads and runs deployed items, and changes nothing. Locks the session's preview too. */
	USE = 'use',
	/** Researches, then proposes a plan for the user's approval. Runs nothing. */
	PLAN = 'plan',
	/** Shown as "Use & build": everything Use does, plus edits, tests and deploys. */
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
		'Not run: the user switched this session to Use mode, which changes nothing, while this call waited for them. Do not retry it; it can run once they switch back to Use & build.',
	/** On a test-run form left open across the switch. */
	testRunRefused: 'Use mode does not run undeployed code. Switch to Use & build to run this test.'
}

/** The chat's tools that ask the user to move the session between Use and Use & build, each
 * with the mode its card leads to. Named here because the card renderer and the picker resolve
 * them by name. */
export const ENTER_BUILD_MODE_TOOL = 'enter_build_mode'
export const ENTER_USE_MODE_TOOL = 'enter_use_mode'
export const WORK_MODE_SWITCH_TARGET = {
	[ENTER_BUILD_MODE_TOOL]: AIWorkMode.BUILD,
	[ENTER_USE_MODE_TOOL]: AIWorkMode.USE
} as const

export type WorkModeSwitchTool = keyof typeof WORK_MODE_SWITCH_TARGET

export function isWorkModeSwitchTool(name: string | undefined): name is WorkModeSwitchTool {
	// hasOwn, not `in`: the name comes from the model, and `in` would accept `toString`.
	return name !== undefined && Object.hasOwn(WORK_MODE_SWITCH_TARGET, name)
}

/** In the shape of plan mode's card copy, whose card these render as. `declined` names the
 * outcome: picking another mode in the picker resolves the card too. */
export const WORK_MODE_SWITCH_CARD_COPY = {
	[ENTER_BUILD_MODE_TOOL]: {
		pending: 'Switch to Use & build?',
		confirm: 'Switch to Use & build',
		reject: 'Stay in Use',
		settled: 'Switched to Use & build',
		declined: 'Not switched to Use & build'
	},
	[ENTER_USE_MODE_TOOL]: {
		pending: 'Switch to Use?',
		confirm: 'Switch to Use',
		reject: 'Stay in Use & build',
		settled: 'Switched to Use',
		declined: 'Not switched to Use'
	}
} as const
