import { ChevronsRight, ClipboardList, Hammer, Hand, Play } from 'lucide-svelte'
import { AIAutonomyMode } from './AIChatManager.svelte'
import { PLAN_MODE_TEXT_COLOR, PLAN_MODE_TRIGGER_CLASS } from './planMode'
import { AIWorkMode } from './workMode'

export type AutonomyAvailability = {
	autoAcceptEditsAvailable: boolean
	autoAcceptToolConfirmationsAvailable: boolean
}

export type AutonomyModeOption = {
	mode: AIAutonomyMode
	label: string
	shortLabel?: string
	icon: typeof Hand
	iconColor: string
	tooltip: (a: AutonomyAvailability) => string
	isAvailable: (a: AutonomyAvailability) => boolean
}

// The one posture available everywhere, so also the fallback for a mode the
// current AI mode does not offer.
const askPermissionOption: AutonomyModeOption = {
	mode: AIAutonomyMode.DEFAULT,
	label: 'Ask permission',
	icon: Hand,
	iconColor: 'text-secondary',
	tooltip: (a) =>
		a.autoAcceptEditsAvailable
			? 'Requires confirmation for edits and tool calls.'
			: 'Requires confirmation for tool calls.',
	isAvailable: () => true
}

// One row per autonomy posture, in picker order, so adding one touches only this
// table. `isAvailable` hides the postures that would do nothing in the current AI
// mode, which is why the picker can be shorter than this list.
const autonomyModeOptions: AutonomyModeOption[] = [
	askPermissionOption,
	{
		mode: AIAutonomyMode.ACCEPT_EDIT,
		label: 'Auto-accept edits',
		icon: ChevronsRight,
		iconColor: 'text-accent',
		tooltip: () =>
			'Automatically accepts script and flow edits. Tool calls still ask for confirmation.',
		isAvailable: (a) => a.autoAcceptEditsAvailable
	},
	{
		mode: AIAutonomyMode.YOLO,
		label: 'Yolo (bypass permissions)',
		shortLabel: 'Yolo',
		icon: ChevronsRight,
		iconColor: 'text-red-500',
		tooltip: (a) =>
			a.autoAcceptEditsAvailable
				? 'Automatically accepts script and flow edits plus tool confirmations.'
				: 'Automatically accepts tool confirmations.',
		isAvailable: (a) => a.autoAcceptToolConfirmationsAvailable
	}
]

export function autonomyModeOption(mode: AIAutonomyMode): AutonomyModeOption {
	return autonomyModeOptions.find((o) => o.mode === mode) ?? askPermissionOption
}

export function availableAutonomyModeOptions(a: AutonomyAvailability): AutonomyModeOption[] {
	return autonomyModeOptions.filter((option) => option.isAvailable(a))
}

/** Falls back to ask-permission when `mode` isn't applicable in the current AI mode
 * (e.g. auto-accept edits while in a mode without edits). */
export function resolveAutonomyMode(mode: AIAutonomyMode, a: AutonomyAvailability) {
	return availableAutonomyModeOptions(a).some((option) => option.mode === mode)
		? mode
		: AIAutonomyMode.DEFAULT
}

export type WorkModeOption = {
	mode: AIWorkMode
	label: string
	description: string
	icon: typeof Hand
	iconColor: string
	/** Tints the whole trigger, not just its icon. Only plan mode needs it. */
	triggerClass?: string
}

/** A session chat's first section, above the postures: what the chat may do. */
export const workModeOptions: WorkModeOption[] = [
	{
		mode: AIWorkMode.USE,
		label: 'Use',
		description: 'Reads and runs. No changes.',
		icon: Play,
		iconColor: 'text-secondary'
	},
	{
		mode: AIWorkMode.PLAN,
		label: 'Plan',
		description: 'Proposes a plan first.',
		icon: ClipboardList,
		iconColor: PLAN_MODE_TEXT_COLOR,
		triggerClass: PLAN_MODE_TRIGGER_CLASS
	},
	{
		mode: AIWorkMode.BUILD,
		label: 'Build',
		description: 'Edits, tests and deploys.',
		icon: Hammer,
		iconColor: 'text-secondary'
	}
]

export function workModeOption(mode: AIWorkMode): WorkModeOption {
	return workModeOptions.find((o) => o.mode === mode) ?? workModeOptions[2]
}
