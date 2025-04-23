import type { FlowStatusModule } from '$lib/gen'

export const NODE = {
	width: 275,
	height: 38,
	gap: {
		horizontal: 40,
		vertical: 50
	}
}

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
				return isDark ? '#2E3440' : 'white'
			} else {
				return isDark ? '#313742' : '#dfe6ee'
			}
	}
}

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
			return getStateColor(state, isDark, nonVirtualItem, isSkipped)

		default:
			if (nonVirtualItem) {
				return isDark ? '#343B46' : '#f6f6f6'
			} else {
				return isDark ? '#343B46' : '#d5dee8'
			}
	}
}
