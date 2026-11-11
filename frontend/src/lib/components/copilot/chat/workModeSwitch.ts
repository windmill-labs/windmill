import { z } from 'zod'
import { createToolDef } from './shared'
import { NONE, type SessionTool } from './sessionCapabilities'
import {
	ENTER_BUILD_MODE_TOOL,
	ENTER_USE_MODE_TOOL,
	WORK_MODE_SWITCH_CARD_COPY,
	WORK_MODE_SWITCH_TARGET,
	type AIWorkMode,
	type WorkModeSwitchTool
} from './workMode'

const switchArgs = z.object({
	reason: z
		.string()
		.describe('One concise sentence on why, shown to the user on the card that asks them.')
})

const DESCRIPTIONS: Record<WorkModeSwitchTool, string> = {
	[ENTER_BUILD_MODE_TOOL]:
		'Asks the user to switch this session from Use to Use & build, the mode that also edits, tests and deploys. Call it when they ask for a change, which Use cannot make: the switch, and the tools that change the workspace, come only if they accept. Not for questions or for running deployed items, which Use already does.',
	[ENTER_USE_MODE_TOOL]:
		'Asks the user to switch this session to Use, which reads and runs deployed items and changes nothing. Call it only when they ask you to stop changing the workspace in this session, never for a single question. The switch happens only if they accept.'
}

// `declined` names the switch, not a mode: picking another mode in the picker declines the card too.
const RESULTS: Record<WorkModeSwitchTool, { switched: string; declined: string }> = {
	[ENTER_BUILD_MODE_TOOL]: {
		switched:
			'The user switched this session to Use & build. The tools that change the workspace are available from your next step: go on with the change they asked for.',
		declined:
			'The user did not switch this session to Use & build. Do not make the change: carry on in the mode the session is in now, and say they can pick Use & build in the mode picker whenever they want it.'
	},
	[ENTER_USE_MODE_TOOL]: {
		switched:
			'The user switched this session to Use: from your next step you read and run deployed items, and change nothing.',
		declined:
			'The user did not switch this session to Use. Carry on in the mode the session is in now.'
	}
}

const NO_LONGER_APPLIES = {
	label: 'Switch no longer applies',
	result:
		'Not run: the session changed mode before the user saw this switch. Carry on in the mode it is in now.'
}

/** What a switch needs from the chat it runs in. */
export interface WorkModeSwitchHost {
	/** Whether the chat offers this switch now, from the mode the session is in. */
	offers(tool: WorkModeSwitchTool): boolean
	setWorkMode(mode: AIWorkMode): void
}

export function workModeSwitchTools(
	host: WorkModeSwitchHost
): Record<WorkModeSwitchTool, SessionTool<any>> {
	return {
		[ENTER_BUILD_MODE_TOOL]: workModeSwitchTool(ENTER_BUILD_MODE_TOOL, host),
		[ENTER_USE_MODE_TOOL]: workModeSwitchTool(ENTER_USE_MODE_TOOL, host)
	}
}

/** The switch happens in `fn`, so only once the user accepted the card: the chat stops offering
 * the switches while confirmations are auto-accepted, and that posture never answers one. */
function workModeSwitchTool(tool: WorkModeSwitchTool, host: WorkModeSwitchHost): SessionTool<any> {
	return {
		requires: NONE,
		def: createToolDef(switchArgs, tool, DESCRIPTIONS[tool]),
		// It changes no workspace item, only this session's mode. Not planModeSafe: a call left
		// over from Use must not take a session out of Plan past its plan's approval.
		readOnlySafe: true,
		requiresConfirmation: true,
		// A card queued behind another call can come up after the picker moved on.
		validateBeforeConfirmation: () => (host.offers(tool) ? undefined : NO_LONGER_APPLIES),
		confirmationMessage: WORK_MODE_SWITCH_CARD_COPY[tool].pending,
		cancellationMessage: RESULTS[tool].declined,
		showDetails: true,
		fn: async () => {
			host.setWorkMode(WORK_MODE_SWITCH_TARGET[tool])
			return RESULTS[tool].switched
		}
	}
}
