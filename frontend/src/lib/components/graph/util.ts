import type { FlowStatusModule } from '$lib/gen'

export const NODE = {
	width: 275,
	height: 34,
	gap: {
		horizontal: 40,
		vertical: 50
	}
}

export function* createIdGenerator(): Generator<number, number, unknown> {
	let id = 0
	while (true) {
		yield id++
	}
}

export function getStateColor(state: FlowStatusModule['type'] | undefined): string {
	const isDark = document.documentElement.classList.contains('dark')
	switch (state) {
		case 'Success':
			return isDark ? '#059669' : 'rgb(193, 255, 216)'
		case 'Failure':
			return isDark ? '#dc2626' : 'rgb(248 113 113)'
		case 'InProgress':
			return isDark ? '#f59e0b' : 'rgb(253, 240, 176)'
		case 'WaitingForEvents':
			return isDark ? '#db2777' : 'rgb(229, 176, 253)'
		case 'WaitingForExecutor':
			return isDark ? '#ea580c' : 'rgb(255, 208, 193)'
		default:
			return isDark ? '#2e3440' : '#fff'
	}
}
