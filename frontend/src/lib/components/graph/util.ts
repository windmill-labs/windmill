import type { FlowStatusModule } from '$lib/gen'
import type { AIModuleAction } from '$lib/components/flows/flowDiff'

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

export const AI_OR_ASSET_NODE_TYPES = ['asset', 'assetsOverflowed', 'newAiTool', 'aiTool']

export type FlowNodeState =
	| FlowStatusModule['type']
	| '_VirtualItem'
	| '_Skipped'
	| '_AIAdded'
	| '_AIModified'
	| '_AIRemoved'
	| '_AIShadowed'
	| undefined

/**
 * Convert AI module action to FlowNodeState
 */
export function aiActionToNodeState(action: AIModuleAction): FlowNodeState {
	switch (action) {
		case 'added':
			return '_AIAdded'
		case 'modified':
			return '_AIModified'
		case 'removed':
			return '_AIRemoved'
		case 'shadowed':
			return '_AIShadowed'
		default:
			return undefined
	}
}

export function getNodeColorClasses(state: FlowNodeState, selected: boolean): FlowNodeColorClasses {
	let outlined = ' outline outline-1 active:outline active:outline-1'

	let defaultStyle = {
		selected: {
			bg: 'bg-surface-accent-selected',
			outline: 'outline-border-selected' + outlined,
			text: 'text-accent',
			badge: 'bg-blue-100 outline-border-selected text-blue-800'
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
			bg: 'bg-orange-200 dark:bg-orange-700',
			outline: 'outline-orange-500' + outlined,
			text: 'text-orange-800 dark:text-orange-200',
			badge: 'bg-orange-100 text-orange-700'
		},
		notSelected: {
			bg: 'bg-orange-100 dark:bg-orange-800',
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
				bg: 'bg-blue-100 dark:bg-blue-950',
				outline: '',
				text: 'text-blue-600 dark:text-blue-200',
				badge: 'bg-blue-200 outline-border-selected text-blue-800'
			}
		},
		Success: {
			selected: {
				bg: 'bg-green-200 dark:bg-green-600',
				outline: 'outline-green-500 dark:outline-green-400' + outlined,
				text: 'text-green-800 dark:text-green-100',
				badge: 'bg-green-100 text-green-700'
			},
			notSelected: {
				bg: 'bg-green-100 dark:bg-green-700',
				outline: '',
				text: 'text-green-700 dark:text-green-100',
				badge: 'bg-green-200 text-green-700'
			}
		},
		Failure: {
			selected: {
				bg: 'bg-red-200 dark:bg-red-600',
				outline: 'outline-red-500' + outlined,
				text: 'text-red-800 dark:text-red-100',
				badge: 'bg-red-100 text-red-700'
			},
			notSelected: {
				bg: 'bg-red-100 dark:bg-red-700',
				outline: '',
				text: 'text-red-700 dark:text-red-200',
				badge: 'bg-red-200 text-red-700'
			}
		},
		InProgress: orangeStyle,
		WaitingForExecutor: orangeStyle,
		WaitingForEvents: {
			selected: {
				bg: 'bg-purple-200 dark:bg-purple-600',
				outline: 'outline-purple-500' + outlined,
				text: 'text-purple-800 dark:text-purple-100',
				badge: 'bg-purple-100 text-purple-700'
			},
			notSelected: {
				bg: 'bg-purple-100 dark:bg-purple-700',
				outline: '',
				text: 'text-purple-700 dark:text-purple-200',
				badge: 'bg-purple-200 text-purple-700'
			}
		},
		// AI Module Action states (distinct shades from execution states)
		_AIAdded: {
			selected: {
				bg: 'bg-green-300 dark:bg-green-800',
				outline: 'outline-green-600 dark:outline-green-500' + outlined,
				text: 'text-green-900 dark:text-green-100',
				badge: 'bg-green-200 text-green-800'
			},
			notSelected: {
				bg: 'bg-green-300 dark:bg-green-900',
				outline: '',
				text: 'text-green-800 dark:text-green-200',
				badge: 'bg-green-300 text-green-800'
			}
		},
		_AIModified: {
			selected: {
				bg: 'bg-orange-300 dark:bg-orange-800',
				outline: 'outline-orange-600' + outlined,
				text: 'text-orange-900 dark:text-orange-100',
				badge: 'bg-orange-200 text-orange-800'
			},
			notSelected: {
				bg: 'bg-orange-300 dark:bg-orange-900',
				outline: '',
				text: 'text-orange-800 dark:text-orange-200',
				badge: 'bg-orange-300 text-orange-800'
			}
		},
		_AIRemoved: {
			selected: {
				bg: 'bg-red-300/50 dark:bg-red-800/50',
				outline: 'outline-red-600' + outlined,
				text: 'text-red-900 dark:text-red-100',
				badge: 'bg-red-200 text-red-800'
			},
			notSelected: {
				bg: 'bg-red-300/50 dark:bg-red-900/50',
				outline: '',
				text: 'text-red-800 dark:text-red-200',
				badge: 'bg-red-300 text-red-800'
			}
		},
		_AIShadowed: {
			selected: {
				bg: 'bg-gray-300/30 dark:bg-gray-600/30 opacity-50',
				outline: 'outline-gray-500' + outlined,
				text: 'text-gray-700 dark:text-gray-300',
				badge: 'bg-gray-200 text-gray-700'
			},
			notSelected: {
				bg: 'bg-gray-300/30 dark:bg-gray-700/30 opacity-50',
				outline: '',
				text: 'text-gray-600 dark:text-gray-400',
				badge: 'bg-gray-300 text-gray-700'
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

/**
 * Calculate the bounding box for a collection of nodes, accounting for CSS offset
 * Also includes expanded subflow nodes when calculating bounds for subflow containers
 * @param containedIds - Array of node IDs to calculate bounds for
 * @param allNodes - Array of all nodes to search for expanded subflow nodes
 * @returns The bounds { minX, minY, maxX, maxY }
 */
export function calculateNodesBoundsWithOffset(
	containedIds: string[],
	allNodes: Array<{
		id: string
		position: { x: number; y: number }
		data?: { offset?: number }
		type: string
	}>
): {
	minX: number
	minY: number
	maxX: number
	maxY: number
} {
	// Find related subflow nodes
	const nodesToCalculate = getAllRelatedSubflowNodes(containedIds, allNodes)

	return nodesToCalculate.reduce(
		(acc, node) => {
			// Account for CSS offset applied by NodeWrapper
			const cssOffset = node.data?.offset ?? 0
			const visualX = node.position.x + cssOffset

			return {
				minX: Math.min(acc.minX, visualX),
				minY: Math.min(acc.minY, node.position.y),
				maxX: Math.max(acc.maxX, visualX + NODE.width),
				maxY: Math.max(acc.maxY, node.position.y + NODE.height)
			}
		},
		{
			minX: Infinity,
			minY: Infinity,
			maxX: -Infinity,
			maxY: -Infinity
		}
	)
}

/**
 * Find all nodes related to the given node IDs, including expanded subflow nodes
 * @param targetNodeIds - Array of node IDs to find related nodes for
 * @param allNodes - Array of all available nodes
 * @returns Array of nodes including original nodes and any related subflow nodes
 */
function getAllRelatedSubflowNodes(
	targetNodeIds: string[],
	allNodes: Array<{
		id: string
		position: { x: number; y: number }
		data?: { offset?: number }
		type: string
	}>
): Array<{
	id: string
	position: { x: number; y: number }
	data?: { offset?: number }
}> {
	const relatedNodeIds = new Set<string>()

	// Add original target nodes
	targetNodeIds.forEach((id) => relatedNodeIds.add(id))

	// For each target node, check if it's a subflow and find expanded nodes
	targetNodeIds.forEach((nodeId) => {
		// Find nodes like "subflow:{nodeId}:*"
		const subflowNodes = allNodes.filter(
			(node) =>
				node.id.startsWith(`subflow:${nodeId}:`) && !AI_OR_ASSET_NODE_TYPES.includes(node.type)
		)

		// Find end node like "{nodeId}-subflow-end"
		const endNode = allNodes.find((node) => node.id === `${nodeId}-subflow-end`)

		// Add all found nodes
		subflowNodes.forEach((node) => relatedNodeIds.add(node.id))
		if (endNode) relatedNodeIds.add(endNode.id)
	})

	// Return actual node objects that exist in allNodes
	return allNodes.filter((node) => relatedNodeIds.has(node.id))
}

/**
 * Generate a random unique ID for notes
 * @returns A random string ID
 */
export function generateId(): string {
	return 'note-' + Math.random().toString(36).substring(2) + Math.random().toString(36).substring(2)
}
