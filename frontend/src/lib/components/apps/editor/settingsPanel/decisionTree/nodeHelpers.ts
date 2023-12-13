import { NODE, type Node } from '$lib/components/graph'

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
	label?: string
	edgeColor?: string
}

export function createNode(nodeConfig: NodeConfig): Node {
	return {
		type: 'node',
		id: nodeConfig.id,
		position: nodeConfig.position || { x: -1, y: -1 },
		data: nodeConfig.data,
		width: nodeConfig.width || NODE.width,
		height: nodeConfig.height || NODE.height,
		borderColor: nodeConfig.borderColor || '#999',
		sourcePosition: 'bottom',
		targetPosition: 'top',
		parentIds: nodeConfig.parentIds || [],
		loopDepth: nodeConfig.loopDepth || 0
	}
}

export function createEdge(edgeConfig: EdgeConfig) {
	console.log('## EDGE ##', edgeConfig.source, '->', edgeConfig.target)

	return {
		id: edgeConfig.id,
		source: edgeConfig.source,
		target: edgeConfig.target,
		label: edgeConfig.label || '',
		edgeColor: edgeConfig.edgeColor || '#999'
	}
}
