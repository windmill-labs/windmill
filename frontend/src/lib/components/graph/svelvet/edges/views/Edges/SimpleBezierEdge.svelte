<script lang="ts">
	import BaseEdge from './BaseEdge.svelte'
	// import { Position } from '../types/utils';
	// // enumerable values (static) set for Position
	// export var Position;
	// (function (Position) {
	//     Position["Left"] = "left";
	//     Position["Right"] = "right";
	//     Position["Top"] = "top";
	//     Position["Bottom"] = "bottom";
	// })(Position || (Position = {}));
	// //
	// // export type CoordinateExtent = [[number, number], [number, number]];
	const Position = { Left: 'left', Right: 'right', Top: 'top', Bottom: 'bottom' }

	import { findStore } from '../../../store/controllers/storeApi'
	import { getAnchorFromEdge } from '../../../edges/controllers/util'

	function calculateControlOffset(distance, curvature) {
		if (distance >= 0) {
			return 0.5 * distance
		} else {
			return curvature * 25 * Math.sqrt(-distance)
		}
	}
	// get the control point for the bezier curve (in the middle of the edge)
	function getControlWithCurvature({ pos, x1, y1, x2, y2, c }) {
		let ctX, ctY
		switch (pos) {
			case Position.Left:
				{
					ctX = x1 - calculateControlOffset(x1 - x2, c)
					ctY = y1
				}
				break
			case Position.Right:
				{
					ctX = x1 + calculateControlOffset(x2 - x1, c)
					ctY = y1
				}
				break
			case Position.Top:
				{
					ctX = x1
					ctY = y1 - calculateControlOffset(y1 - y2, c)
				}
				break
			case Position.Bottom:
				{
					ctX = x1
					ctY = y1 + calculateControlOffset(y2 - y1, c)
				}
				break
		}
		return [ctX, ctY]
	}
	// returns string to pass into edge 'path' svg d attribute (where to be drawn)
	// referenced from ReactFlow.dev
	function getSimpleBezierPath({
		sourceX,
		sourceY,
		sourcePosition = Position.Bottom,
		targetX,
		targetY,
		targetPosition = Position.Top,
		curvature = 0.25
	}) {
		const [sourceControlX, sourceControlY] = getControlWithCurvature({
			pos: sourcePosition,
			x1: sourceX,
			y1: sourceY,
			x2: targetX,
			y2: targetY,
			c: curvature
		})
		const [targetControlX, targetControlY] = getControlWithCurvature({
			pos: targetPosition,
			x1: targetX,
			y1: targetY,
			x2: sourceX,
			y2: sourceY,
			c: curvature
		})
		return `M${sourceX},${sourceY} C${sourceControlX},${sourceControlY} ${targetControlX},${targetControlY} ${targetX},${targetY}`
	}
	// determining center of the bezier curve to know where to place the bezier edge text label
	function getSimpleBezierCenter({
		sourceX,
		sourceY,
		sourcePosition = Position.Bottom,
		targetX,
		targetY,
		targetPosition = Position.Top,
		curvature = 0.25
	}) {
		const [sourceControlX, sourceControlY] = getControlWithCurvature({
			pos: sourcePosition,
			x1: sourceX,
			y1: sourceY,
			x2: targetX,
			y2: targetY,
			c: curvature
		})
		const [targetControlX, targetControlY] = getControlWithCurvature({
			pos: targetPosition,
			x1: targetX,
			y1: targetY,
			x2: sourceX,
			y2: sourceY,
			c: curvature
		})
		// cubic bezier t=0.5 mid point, not the actual mid point, but easy to calculate
		// https://stackoverflow.com/questions/67516101/how-to-find-distance-mid-point-of-bezier-curve
		const centerX =
			sourceX * 0.125 + sourceControlX * 0.375 + targetControlX * 0.375 + targetX * 0.125
		const centerY =
			sourceY * 0.125 + sourceControlY * 0.375 + targetControlY * 0.375 + targetY * 0.125
		const xOffset = Math.abs(centerX - sourceX)
		const yOffset = Math.abs(centerY - sourceY)
		return [centerX, centerY, xOffset, yOffset]
	}
	export let canvasId: string
	export let edgeId: string
	const store = findStore(canvasId)
	const { nodesStore, edgesStore, anchorsStore } = store
	let edge
	$: edge = $edgesStore[edgeId]

	let params
	$: {
		const store = findStore(canvasId)
		const sourceAnchor = getAnchorFromEdge(store, edge.id, 'source')
		const targetAnchor = getAnchorFromEdge(store, edge.id, 'target')
		const mapAngle = { 0: 'right', 90: 'top', 180: 'left', 270: 'bottom' }
		params = {
			sourceX: edge.sourceX,
			sourceY: edge.sourceY,
			sourcePosition: mapAngle[sourceAnchor.angle],
			targetX: edge.targetX,
			targetY: edge.targetY,
			targetPosition: mapAngle[targetAnchor.angle],
			curvature: 0.25
		}
	}

	// pass in params to function that returns a string value for SVG path d attribute (where to be drawn)
	$: path = getSimpleBezierPath(params)
	$: [centerX, centerY] = getSimpleBezierCenter(params)
	// pass necessary values to BaseEdge component
	// BaseEdge renders a 'base' path that can be customized by parent Edge components
	$: baseEdgeProps = {
		...edge,
		path: path,
		centerX: centerX,
		centerY: centerY
	}
</script>

<BaseEdge {baseEdgeProps} {canvasId} />
