import { ChevronsRight, ClipboardList, Hand } from 'lucide-svelte'
import { AIAutonomyMode } from './AIChatManager.svelte'
import { PLAN_MODE_TEXT_COLOR, PLAN_MODE_TRIGGER_CLASS } from './planMode'

export type AutonomyAvailability = {
	autoAcceptEditsAvailable: boolean
	autoAcceptToolConfirmationsAvailable: boolean
	planModeAvailable: boolean
}

export type AutonomyModeOption = {
	mode: AIAutonomyMode
	label: string
	shortLabel?: string
	icon: typeof Hand
	iconColor: string
	/** Tints the whole trigger, not just its icon. Only plan mode needs it. */
	triggerClass?: string
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
	{
		mode: AIAutonomyMode.PLAN,
		label: 'Plan (read-only)',
		shortLabel: 'Plan',
		icon: ClipboardList,
		iconColor: PLAN_MODE_TEXT_COLOR,
		triggerClass: PLAN_MODE_TRIGGER_CLASS,
		tooltip: () =>
			'Read-only: the assistant researches and drafts a plan for your approval before it can change anything.',
		isAvailable: (a) => a.planModeAvailable
	},
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
