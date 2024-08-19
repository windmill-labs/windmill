import { NODE } from '$lib/components/graph'
import { type Node } from '@xyflow/svelte'

interface NodeConfig {
	id: string
	type?: string
	position?: { x: number; y: number }
	data: {
		custom: {
			component: any
			props?: any
			cb?: (e: string, detail: any) => void
		}
	}
	width?: number
	height?: number
	borderColor?: string
	sourcePosition?: string
	targetPosition?: string
	parentIds?: string[]
	loopDepth?: number
}

interface EdgeConfig {
	id: string
	source: string
	target: string
	label?: string | undefined
	edgeColor?: string | undefined
}

export function createNode(nodeConfig: NodeConfig): Node {
	return {
		type: 'node',
		id: nodeConfig.id,
		position: nodeConfig.position || { x: -1, y: -1 },
		data: nodeConfig.data,
		width: nodeConfig.width || NODE.width,
		height: nodeConfig.height || NODE.height
	}
}

export function createEdge(edgeConfig: EdgeConfig) {
	return {
		id: `edge-${edgeConfig.id}`,
		source: edgeConfig.source,
		target: edgeConfig.target,
		label: edgeConfig.label || '',
		edgeColor: edgeConfig.edgeColor || '#999',
		type: 'bezier' as const,
		labelBgColor: 'white',
		arrow: false,
		animate: false,
		noHandle: true
	}
}
