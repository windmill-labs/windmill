/*


Important functions (in order of how likely I think you are to use them):

findStore(canvasId: string)
- Input: canvasId. There can be multiple Svelvet canvases on the same page, and each has their own store
- Returns: store
- Notes: You will need to call this function on every component you make

getNodes(store, filter)
- Description: this function returns of list of Node objects
- Input: store: the store where you get the list of Node objects from
-        filter: an object where you can filter the array. For example, if you want to find nodes with an id of 'sdf-2d3fs' and positionX=35, then you
                 would set filter = {id: 'sdf-2d3fs, positionX: 35}. 
                 Right now, you can only test for equality.
- Notes: This is a very mongoose way of retrieving information. Given the relational nature of our stores, maybe it would be better to use an SQL-like syntax
         This array is non-reactive (ie, you cannot use information from this array to force a re-render of a Svelte component)

createStoreEmpty(canvasId: string)
- Input: canvasId. 
- Returns: store
- Notes. This should be called once every time you initialize a new Svelvet canvas, (ie, only in the Svelvet.svelte file)

populateSvelvetStoreFromUserInput(canvasId, nodes, edges)
- canvasId: this the the canvasId of the Svelvet component you are creating a store for
- nodes: this is an array of objects containing node info that is defined by the user. NOTE THAT THE STRUCTURE DIFFERS FROM THE NODES CLASS
         The whole point of populateSvelvetStoreFromUserInput is to convert nodes into proper Svelvet Node objects. An example of nodes is in 
         $routes/testingplayground/index.svelte
- edges: same as nodes, this is an array of objects containing edge info THAT IS DIFFERENT FROM THE EDGE CLASS.
- Returns: store
*/
import { stores } from '../models/store'
import { writable, get } from 'svelte/store'

import type { StoreType } from '../types/types'
import type { UserNodeType, UserEdgeType } from '../../types/types'
import { populateAnchorsStore, populateNodesStore, populateEdgesStore } from './util'
import { populateCollapsibleStore } from '../../collapsible/controllers/util'

/**
 * findStore is going to return the target Svelvet store with the canvasId provided as argument.
 * There can be multiple Svelvet canvases on the same page, and each has their own store with a unique canvasId.
 * @param canvasId The canvasId of a Svelvet component
 * @returns The store of a Svelvet component that matches the canvasId
 */
export function findStore(canvasId: string): StoreType {
	return stores[canvasId]
}

/**
 * createStoreEmpty will initialize a new Svelvet store with a unique canvasId.
 * If you have multiple Svelvet components on the page, the stores object will look like the following example:
 * const stores = \{
 *                  canvasId-1: store of Svelvet component 1,
 *                  canvasId-2: store of Svelvet component 2,
 *                  canvasId-3: store of Svelvet component 3,
 *                \}
 * Notes: This should be called once every time you initialize a new Svelvet canvas, (ie, only in the Svelvet.svelte file).
 * This function will initialize an empty store for the Svelvet component and should be followed by invoking populateSvelvetStoreFromUserInput to populate all the initial state from the user input.
 *
 * @param canvasId The canvasId of the newly created Svelvet component
 * @returns An empty store for the newly created Svelvet component.
 */
export function createStoreEmpty(canvasId: string): StoreType {
	stores[canvasId] = {
		nodesStore: writable({}),
		edgesStore: writable({}),
		anchorsStore: writable({}),
		potentialAnchorsStore: writable({}),
		widthStore: writable(600),
		heightStore: writable(600),
		backgroundStore: writable(false),
		movementStore: writable(true),
		nodeSelected: writable(false),
		nodeIdSelected: writable(-1),
		d3Scale: writable(1),
		options: writable({}),
		temporaryEdgeStore: writable([]),
		nodeCreate: writable(false), // this option sets whether the "nodeEdit" feature is enabled
		boundary: writable(false),
		edgeEditModal: writable(null), // this is used for edgeEditModal feature. When an edge is right clicked, store.edgeEditModal is set to the edgeId string. This causes a modal to be rendered
		collapsibleStore: writable([]), // this is used for the collaspsible node feature. If the feature is enabled, store.collapsible will be populated with Collapsible objects which will track whether the node should be displayed or not
		collapsibleOption: writable(false),
		lockedOption: writable(false),
		editableOption: writable(false), // true if you want nodes/edges to be editable. See feature editEdges
		d3ZoomParameters: writable({}), // this stores d3 parameters x, y, and zoom. This isn't used for anything other than giving users a way to access d3 zoom parameters if they want to build on top of Svelvet
		highlightEdgesOption: writable(true) // option to turn on/off highlightable edges
	}
	return stores[canvasId]
}

/**
 * populateSvelvetStoreFromUserInput will populate all the states and set these states into the Svelvet store initialized by invoking createStoreEmpty
 *
 * @param canvasId The canvasId of the Svelvet component you are creating a store for
 * @param nodes This is an array of objects containing node info that is defined by the user. NOTE THAT THE STRUCTURE DIFFERS FROM THE NODES CLASS. The whole point of populateSvelvetStoreFromUserInput is to convert nodes into proper Svelvet Node objects. An example of nodes is in $routes/testingplayground/index.svelte
 * @param edges Same as nodes, this is an array of objects containing edge info THAT IS DIFFERENT FROM THE EDGE CLASS.
 */
export function populateSvelvetStoreFromUserInput(
	canvasId: string,
	nodes: UserNodeType[],
	edges: UserEdgeType[]
): void {
	// find the store
	const store = findStore(canvasId)

	// populate store.nodesStore with user nodes
	populateNodesStore(store, nodes, canvasId)
	// populate store.anchorsStore with anchors. Note the userdoes not explictly define anchors; anchors are calculated from the edges
	populateAnchorsStore(store, nodes, edges, canvasId)
	// populate edges
	populateEdgesStore(store, edges, canvasId)

	// populatate collapsible objects if "collapsible" feature is turned on
	if (get(store.collapsibleOption)) populateCollapsibleStore(store, nodes, edges, canvasId)
}
