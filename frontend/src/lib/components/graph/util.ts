import type { FlowStatusModule } from '$lib/gen'
import { getCssColor } from '$lib/utils'

export const NODE = {
	width: 275,
	height: 34,
	gap: {
		horizontal: 50,
		vertical: 62
	}
}

/**
 * @deprecated Use getNodeColorClasses instead
 */
export function getStateColor(
	state: FlowStatusModule['type'] | undefined,
	isDark: boolean,
	nonVirtualItem?: boolean,
	isSkipped?: boolean
): string {
	switch (state) {
		case 'Success':
			if (isSkipped) {
				return isDark ? '#1E3A8A' : 'rgb(191, 219, 254)'
			} else {
				return isDark ? '#059669' : 'rgb(193, 255, 216)'
			}
		case 'Failure':
			return isDark ? '#dc2626' : 'rgb(248 113 113)'
		case 'InProgress':
			return isDark ? '#f59e0b' : 'rgb(253, 240, 176)'
		case 'WaitingForEvents':
			return isDark ? '#75599E' : '#c4b5fd'
		case 'WaitingForExecutor':
			return isDark ? '#ea580c' : 'rgb(255, 208, 193)'
		default:
			if (nonVirtualItem) {
				return getCssColor('surface-tertiary')
			} else {
				return getCssColor('component-virtual-node')
			}
	}
}

/**
 * @deprecated Use getNodeColorClasses instead
 */
export function getStateHoverColor(
	state: FlowStatusModule['type'] | undefined,
	isDark: boolean,
	nonVirtualItem?: boolean,
	isSkipped?: boolean
): string {
	switch (state) {
		case 'Success':
		case 'Failure':
		case 'InProgress':
		case 'WaitingForEvents':
		case 'WaitingForExecutor':
		default:
			return getStateColor(state, isDark, nonVirtualItem, isSkipped)
	}
}

export type FlowNodeColorClasses = {
	text: string
	bg: string
	outline: string
	badge: string
}
export type FlowNodeState = FlowStatusModule['type'] | '_VirtualItem' | '_Skipped' | undefined

export function getNodeColorClasses(state: FlowNodeState, selected: boolean): FlowNodeColorClasses {
	let outlined = ' outline outline-1 active:outline active:outline-1'

	let defaultStyle = {
		selected: {
			bg: 'bg-surface-accent-selected',
			outline: 'outline-luminance-blue-300' + outlined,
			text: 'text-accent',
			badge: 'bg-blue-100 outline-luminance-blue-300 text-accent'
		},
		notSelected: {
			bg: 'bg-surface-tertiary',
			outline: '',
			text: 'text-emphasis',
			badge: 'bg-component-virtual-node text-emphasis'
		}
	} satisfies Record<any, FlowNodeColorClasses>
	let orangeStyle = {
		selected: {
			bg: 'bg-orange-200',
			outline: 'outline-orange-500' + outlined,
			text: 'text-orange-800',
			badge: ''
		},
		notSelected: {
			bg: 'bg-orange-100',
			outline: '',
			text: 'text-orange-700',
			badge: ''
		}
	} satisfies Record<any, FlowNodeColorClasses>
	let map = {
		_VirtualItem: {
			selected: defaultStyle.selected,
			notSelected: {
				bg: 'bg-component-virtual-node',
				outline: '',
				text: 'text-emphasis'
			}
		},
		_Skipped: {
			selected: defaultStyle.selected,
			notSelected: {
				bg: 'bg-blue-100',
				outline: '',
				text: 'text-blue-600',
				badge: 'bg-blue-200 outline-luminance-blue-300 text-accent'
			}
		},
		Success: {
			selected: {
				bg: 'bg-green-200',
				outline: 'outline-green-500' + outlined,
				text: 'text-green-800',
				badge: 'bg-green-100 text-green-700'
			},
			notSelected: {
				bg: 'bg-green-100',
				outline: '',
				text: 'text-green-700',
				badge: 'bg-green-200 text-green-700'
			}
		},
		Failure: {
			selected: {
				bg: 'bg-red-200',
				outline: 'outline-red-500' + outlined,
				text: 'text-red-800',
				badge: 'bg-red-100 text-red-700'
			},
			notSelected: {
				bg: 'bg-red-100',
				outline: '',
				text: 'text-red-700',
				badge: 'bg-red-200 text-red-700'
			}
		},
		InProgress: orangeStyle,
		WaitingForExecutor: orangeStyle,
		WaitingForEvents: {
			selected: {
				bg: 'bg-purple-200',
				outline: 'outline-purple-500' + outlined,
				text: 'text-purple-800',
				badge: 'bg-purple-100 text-purple-700'
			},
			notSelected: {
				bg: 'bg-purple-100',
				outline: '',
				text: 'text-purple-700',
				badge: 'bg-purple-200 text-purple-700'
			}
		},
		default: defaultStyle
	} as Record<
		NonNullable<FlowNodeState> | 'default',
		Record<'selected' | 'notSelected', FlowNodeColorClasses>
	>

	let r =
		map[state ?? 'default']?.[selected ? 'selected' : 'notSelected'] ??
		defaultStyle[selected ? 'selected' : 'notSelected']
	r.bg += ' transition-colors'
	r.outline += ' transition-colors'
	r.text += ' transition-colors'
	r.badge = r.badge ?? ''

	return r
}
