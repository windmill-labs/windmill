/** this is where we create our node store */
import type { NodeType } from '../../store/types/types'
import { get } from 'svelte/store'
import { stores } from '../../store/models/store'
/** A Node class that implements NodeType interface
 * @param {string} id The id of the Node
 * @param {number} positionX The X-axis position of the Node (left top corner of the Node)
 * @param {number} positionY The Y-axis position of the Node (left top corner of the Node)
 * @param {number} width The width of the Node
 * @param {number} height The height of the Node
 * @param {string} bgColor The background color of the node
 * @param {object} data A data object that user can specify; possible keys are 'label' and 'custom';
 * @param {string} canvasId The canvasId of the Svelvet component that the instantiated Node will be on.
 * @param {string} borderColor The border color of the Node
 * @param {boolean} image A boolean set to true if the Node needs to display an image
 * @param {string} src The src link for the image; image and src are closely tied and a src link is only needed when image sets to true
 * @param {string} textColor The color of the text in the Node
 * @param {string} borderRadius The border radius of the Node
 * @param {string} childNodes An array of node ids that will be grouped as child nodes of this Node. This is for the GroupNodes feature. The current implementation of this feature works one way but not the other (when you drag the parent node, the child nodes will move as a group but when you drag the child node, the parent node would not move along)
 * @param {string} className The custom class name if user specifies. This is for the custom className feature for Node.
 */
export class Node implements NodeType {
	constructor(
		public id: string,
		public positionX: number,
		public positionY: number,
		public width: number,
		public height: number,
		public bgColor: string,
		public data: object,
		public canvasId: string,
		public borderColor: string,
		public image: boolean,
		public src: string,
		public textColor: string,
		public borderRadius: number,
		public childNodes: string[],
		public className: string,
		public clickCallback: Function
	) {}

	/**
	 * setPositionFromMovement will update the positionX and positionY of the Node when user drags a Node around on the canvas, reflect the changes in real time in the nodesStore, and also cascade the changes to all relative elements like Anchors and Edges.
	 * @param {number} movementX The mouse movement value on the X-axis
	 * @param {number} movementY The mouse movement value on the Y-axis
	 */
	setPositionFromMovement(movementX: number, movementY: number) {
		const { nodesStore, anchorsStore, potentialAnchorsStore, boundary, lockedOption } =
			stores[this.canvasId]

		if (get(lockedOption)) return // don't do anything if locked is enabled

		// boundary sets the boundary of the canvas, or else it is false if their is no boundary
		// check if out of bounds, and if so return without doing anything
		const boundaryObj = get(boundary)
		if (
			typeof boundaryObj === 'object' &&
			(this.positionX + this.width + movementX >= boundaryObj.x ||
				this.positionY + this.height + movementY >= boundaryObj.y ||
				this.positionY + movementY <= 0 ||
				this.positionX + movementX <= 0)
		)
			return
		//update all necessary data
		this.positionX += movementX
		this.positionY += movementY

		// update children
		nodesStore.update((nodes) => {
			if (this.childNodes)
				for (const childNodeId of this.childNodes)
					nodes[childNodeId].setPositionFromMovement(movementX, movementY)
			return { ...nodes }
		})

		//update all the anchors on the node in the anchorsStore
		anchorsStore.update((anchors) => {
			for (const anchorId in anchors) {
				if (anchors[anchorId].nodeId === this.id) {
					anchors[anchorId].setPositionFromNode()
				}
			}
			return { ...anchors }
		})

		//update all the anchors on the node in the anchorsStore
		potentialAnchorsStore.update((anchors) => {
			for (const anchorId in anchors) {
				if (anchors[anchorId].nodeId === this.id) {
					anchors[anchorId].callback() // we don't have to worry about setting partner anchors/etc;
				}
			}
			return { ...anchors }
		})
	}

	/**
	 * setSizeFromMovement will update the width and height of the Node when user resizes the Node by dragging at the right bottom corner (where the ResizedNode attached), reflect the changes in real time in the nodesStore, and also cascade the changes to all relative elements like Anchors and potential Anchors.
	 *
	 * @param movementX The mouse movement value on the X-axis
	 * @param movementY The mouse movement value on the Y-axis
	 */
	setSizeFromMovement(movementX: number, movementY: number) {
		this.width += movementX
		this.height += movementY

		const { anchorsStore, potentialAnchorsStore } = stores[this.canvasId]

		//Updates the anchor so it follows the node's position as the dimensions change
		anchorsStore.update((anchors) => {
			for (const anchorId in anchors) {
				if (anchors[anchorId].nodeId === this.id) {
					anchors[anchorId].setPositionFromNode()
					//anchors[anchorId].setPosition(movementX, movementY);
				}
			}
			return { ...anchors }
		})

		//update all the anchors on the node in the anchorsStore
		potentialAnchorsStore.update((anchors) => {
			for (const anchorId in anchors) {
				if (anchors[anchorId].nodeId === this.id) {
					anchors[anchorId].callback() // we don't have to worry about setting partner anchors/etc;
				}
			}
			return { ...anchors }
		})
	}

	/**
	 * setExportableData is going to construct an object that holds all the node data that can be exported. This method is used for Exporting Diagrams feature.
	 *
	 * @returns An object with all the exportable data of the Node. The format of this object should align with the original format of node data user provided.
	 */
	setExportableData() {
		const exportableData = {
			id: this.id,
			// canvasId: this.canvasId,
			width: this.width,
			height: this.height,
			position: { x: this.positionX, y: this.positionY },
			data: this.data,
			bgColor: this.bgColor,
			borderColor: this.borderColor,
			textColor: this.textColor,
			borderRadius: this.borderRadius,
			image: this.image,
			src: this.src,
			childNodes: this.childNodes,
			customClassName: this.className
		}

		return exportableData
	}
}
