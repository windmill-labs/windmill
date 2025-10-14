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
}
export type FlowNodeState = FlowStatusModule['type'] | '_VirtualItem' | '_Skipped' | undefined

export function getNodeColorClasses(state: FlowNodeState, selected: boolean): FlowNodeColorClasses {
	let outlined = ' outline outline-1 active:outline active:outline-1'
	// return {
	// 	bg: 'bg-red-100',
	// 	outline: 'outline-red-500' + outlined,
	// 	text: 'text-red-800'
	// }
	switch (state) {
		case '_VirtualItem':
			if (!selected) {
				return {
					bg: 'bg-component-virtual-node',
					outline: '',
					text: 'text-emphasis'
				}
			}
		default:
			if (selected) {
				return {
					bg: 'bg-surface-accent-selected',
					outline: 'outline-luminance-blue-300' + outlined,
					text: 'text-accent'
				}
			} else {
				return {
					bg: 'bg-surface-tertiary',
					outline: '',
					text: 'text-emphasis'
				}
			}
	}
}
