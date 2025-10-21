import type { FlowStatusModule } from '$lib/gen'

export const NODE = {
	width: 275,
	height: 34,
	gap: {
		horizontal: 50,
		vertical: 62
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
			bg: 'bg-surface-accent-selected/30',
			outline: 'outline-luminance-blue-300 dark:outline-luminance-blue-200' + outlined,
			text: 'text-accent',
			badge: 'bg-blue-100 outline-luminance-blue-300 text-blue-800'
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
			bg: 'bg-orange-200 dark:bg-[#905036]',
			outline: 'outline-orange-500' + outlined,
			text: 'text-orange-800 dark:text-orange-200',
			badge: 'bg-orange-100 text-orange-700'
		},
		notSelected: {
			bg: 'bg-orange-100 dark:bg-[#654338]',
			outline: '',
			text: 'text-orange-700 dark:text-orange-300',
			badge: 'bg-orange-200 text-orange-700'
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
				bg: 'bg-blue-100 dark:bg-[#2D3B63]',
				outline: '',
				text: 'text-blue-600 dark:text-blue-200',
				badge: 'bg-blue-200 outline-luminance-blue-300 text-blue-800'
			}
		},
		Success: {
			selected: {
				bg: 'bg-green-200 dark:bg-[#329E6A]',
				outline: 'outline-green-500 dark:outline-green-400' + outlined,
				text: 'text-green-800 dark:text-green-100',
				badge: 'bg-green-100 text-green-700'
			},
			notSelected: {
				bg: 'bg-green-100 dark:bg-[#3B7B5B]',
				outline: '',
				text: 'text-green-700 dark:text-green-100',
				badge: 'bg-green-200 text-green-700'
			}
		},
		Failure: {
			selected: {
				bg: 'bg-red-200 dark:bg-[#AA3E47]',
				outline: 'outline-red-500' + outlined,
				text: 'text-red-800 dark:text-red-100',
				badge: 'bg-red-100 text-red-700'
			},
			notSelected: {
				bg: 'bg-red-100 dark:bg-[#81383F]',
				outline: '',
				text: 'text-red-700 dark:text-red-200',
				badge: 'bg-red-200 text-red-700'
			}
		},
		InProgress: orangeStyle,
		WaitingForExecutor: orangeStyle,
		WaitingForEvents: {
			selected: {
				bg: 'bg-purple-200 dark:bg-[#8243BB]',
				outline: 'outline-purple-500' + outlined,
				text: 'text-purple-800 dark:text-purple-100',
				badge: 'bg-purple-100 text-purple-700'
			},
			notSelected: {
				bg: 'bg-purple-100 dark:bg-[#5F417E]',
				outline: '',
				text: 'text-purple-700 dark:text-purple-200',
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
