import { FlowStatusModule, type FlowModule } from '$lib/gen'
import MapItem from '../flows/map/MapItem.svelte'
import type { GraphModuleState } from './model'

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

export function getStateColor(state: FlowStatusModule.type | undefined): string {
	const isDark = document.documentElement.classList.contains('dark')
	switch (state) {
		case FlowStatusModule.type.SUCCESS:
			return isDark ? '#059669' : 'rgb(193, 255, 216)'
		case FlowStatusModule.type.FAILURE:
			return isDark ? '#dc2626' : 'rgb(248 113 113)'
		case FlowStatusModule.type.IN_PROGRESS:
			return isDark ? '#f59e0b' : 'rgb(253, 240, 176)'
		case FlowStatusModule.type.WAITING_FOR_EVENTS:
			return isDark ? '#db2777' : 'rgb(229, 176, 253)'
		case FlowStatusModule.type.WAITING_FOR_EXECUTOR:
			return isDark ? '#ea580c' : 'rgb(255, 208, 193)'
		default:
			return isDark ? '#2e3440' : '#fff'
	}
}

export function flowModuleToNode(
	parentIds: string[],
	mod: FlowModule,
	edgeLabel: string | undefined,
	annotation: string | undefined,
	loopDepth: number,
	insertableEnd: boolean,
	branchable: boolean,
	modules: FlowModule[],
	callbackHandler: (event: string, detail: any) => void,
	flowModuleStates: Record<string, GraphModuleState>,
	insertable: boolean,
	moving: boolean,
	disableAi: boolean
) {
	return {
		type: 'node',
		id: mod.id,
		position: { x: -1, y: -1 },
		data: {
			custom: {
				component: MapItem,
				props: {
					trigger: parentIds.length == 0,
					mod,
					insertable,
					insertableEnd,
					branchable,
					duration_ms: flowModuleStates?.[mod.id]?.duration_ms,
					bgColor: getStateColor(flowModuleStates?.[mod.id]?.type),
					annotation,
					modules,
					moving,
					disableAi
				},
				cb: callbackHandler
			}
		},
		width: NODE.width,
		height: NODE.height,
		parentIds,
		sourcePosition: 'bottom',
		targetPosition: 'top',
		edgeLabel,
		loopDepth
	}
}
