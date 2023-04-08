import type {
	App,
	BaseAppComponent,
	ConnectingInput,
	EditorBreakpoint,
	FocusedGrid,
	GridItem
} from '../types'
import {
	ccomponents,
	components,
	getRecommendedDimensionsByComponent,
	type AppComponent,
	type BaseComponent,
	type InitialAppComponent
} from './component'
import { gridColumns } from '../gridUtils'
import { allItems } from '../utils'
import type { Output, World } from '../rx'
import gridHelp from '../svelte-grid/utils/helper'
import type { FilledItem } from '../svelte-grid/types'
import type { EvalAppInput, StaticAppInput } from '../inputType'
import { get, type Writable } from 'svelte/store'
import { sendUserToast } from '$lib/utils'
import { getNextId } from '$lib/components/flows/idUtils'

export function dfs(
	grid: GridItem[],
	id: string,
	subgrids: Record<string, GridItem[]>
): string[] | undefined {
	for (const item of grid) {
		if (item.id === id) {
			return [id]
		} else if (item.data.type == 'tablecomponent' && item.data.actionButtons.find((x) => x.id)) {
			return [item.id, id]
		} else {
			for (let i = 0; i < (item.data.numberOfSubgrids ?? 0); i++) {
				const res = dfs(subgrids[`${item.id}-${i}`], id, subgrids)
				if (res) {
					return [item.id, ...res]
				}
			}
		}
	}
	return undefined
}

export function selectId(
	e: PointerEvent,
	id: string,
	selectedComponent: Writable<string[] | undefined>,
	app: App
) {
	// this ensure handleClickOutside are triggered
	window.dispatchEvent(
		new MouseEvent('click', {
			view: window,
			bubbles: true,
			cancelable: true
		})
	)
	if (e.shiftKey) {
		selectedComponent.update((old) => {
			if (old && old?.[0]) {
				if (findGridItemParentGrid(app, old[0]) != findGridItemParentGrid(app, id)) {
					sendUserToast('Cannot multi select items from different grids', true)
					return old
				}
			}
			if (old == undefined) {
				return [id]
			}
			if (old.includes(id)) {
				return old
			}
			return [...old, id]
		})
	} else {
		if (get(selectedComponent)?.includes(id)) {
			return
		} else {
			selectedComponent.set([id])
		}
	}
}

function findGridItemById(
	root: GridItem[],
	subGrids: Record<string, GridItem[]> | undefined,
	id: string
): GridItem | undefined {
	for (const gridItem of allItems(root, subGrids)) {
		if (gridItem.id === id) {
			return gridItem
		}
	}
	return undefined
}

export function findGridItemParentGrid(app: App, id: string): string | undefined {
	const gridItem = app.grid.find((x) => x.id === id)
	if (gridItem || app.subgrids === undefined) {
		return undefined
	} else {
		for (const key of Object.keys(app.subgrids ?? {})) {
			const subGrid = app.subgrids[key]
			const gridItemIdx = subGrid.findIndex((x) => x.id === id)
			if (gridItemIdx > -1) {
				return key
			}
		}
	}
}

export function allsubIds(app: App, parentId: string): string[] {
	let item = findGridItem(app, parentId)
	if (!item?.data.numberOfSubgrids) {
		let subIds = [parentId]
		if (item) {
			if (item.data.type === 'tablecomponent') {
				subIds.push(...item.data.actionButtons?.map((x) => x.id))
			}
		}
		return subIds
	}
	return getAllSubgridsAndComponentIds(app, item?.data)[1]
}

export function findGridItem(app: App, id: string): GridItem | undefined {
	return findGridItemById(app.grid, app.subgrids, id)
}

export function getNextGridItemId(app: App): string {
	const subgridsKeys = allItems(app.grid, app.subgrids).map((x) => x.id)
	const withoutDash = subgridsKeys.map((element) => element.split('-')[0])
	const id = getNextId([...new Set(withoutDash)])

	return id
}

export function getAllRecomputeIdsForComponent(app: App, id: string | undefined) {
	if (!app || !id) return []
	const items = allItems(app.grid, app.subgrids)

	const recomputedBy: string[] = []

	items.forEach((item) => {
		if (item.data.type === 'buttoncomponent') {
			if (item.data.recomputeIds?.includes(id)) {
				recomputedBy.push(item.id)
			}
		}
	})

	return recomputedBy
}

export function createNewGridItem(
	grid: GridItem[],
	id: string,
	data: AppComponent,
	columns?: Record<number, any>
): GridItem {
	const newComponent = {
		fixed: false,
		x: 0,
		y: 0
	}

	let newData: AppComponent = JSON.parse(JSON.stringify(data))
	newData.id = id

	const newItem: GridItem = {
		data: newData,
		id: id
	}

	gridColumns.forEach((column) => {
		if (!columns) {
			const rec = getRecommendedDimensionsByComponent(newData.type, column)

			newItem[column] = {
				...newComponent,
				w: rec.w,
				h: rec.h
			}
		} else {
			newItem[column] = columns[column]
		}
		const position = gridHelp.findSpace(newItem, grid, column) as { x: number; y: number }
		newItem[column] = { ...newItem[column], ...position }
	})

	return newItem
}

export function getGridItems(app: App, focusedGrid: FocusedGrid | undefined): GridItem[] {
	if (!focusedGrid) {
		return app.grid
	} else {
		const { parentComponentId, subGridIndex } = focusedGrid
		const key = `${parentComponentId}-${subGridIndex ?? 0}`
		return app?.subgrids?.[key] ?? []
	}
}

function cleanseValue(key: string, value: { type: 'eval' | 'static'; value?: any; expr?: string }) {
	if (!value) {
		return [key, undefined]
	}
	if (value.type === 'static') {
		return [key, { type: value.type, value: value.value }]
	} else {
		return [key, { type: value.type, expr: value.expr }]
	}
}
export function appComponentFromType<T extends keyof typeof components>(
	type: T
): (id: string) => BaseAppComponent & BaseComponent<T> {
	return (id: string) => {
		const init = JSON.parse(JSON.stringify(ccomponents[type].initialData)) as InitialAppComponent
		return {
			type,
			//TODO remove tooltip and onlyStatic from there
			configuration: Object.fromEntries(
				Object.entries(init.configuration).map(([key, value]) => {
					if (value.type != 'oneOf') {
						return cleanseValue(key, value)
					} else {
						return [
							key,
							{
								type: value.type,
								selected: value.selected,
								configuration: Object.fromEntries(
									Object.entries(value.configuration).map(([key, val]) => [
										key,
										Object.fromEntries(
											Object.entries(val).map(([key, val]) => cleanseValue(key, val))
										)
									])
								)
							}
						]
					}
				})
			),
			componentInput: init.componentInput,
			panes: init.panes,
			tabs: init.tabs,
			customCss: {},
			recomputeIds: init.recomputeIds ? [] : undefined,
			actionButtons: init.actionButtons ? [] : undefined,
			numberOfSubgrids: init.numberOfSubgrids,
			horizontalAlignment: init.horizontalAlignment,
			verticalAlignment: init.verticalAlignment,
			id
		}
	}
}
export function insertNewGridItem(
	app: App,
	builddata: (id: string) => AppComponent,
	focusedGrid: FocusedGrid | undefined,
	columns?: Record<string, any>,
	keepId?: string
): string {
	const id = keepId ?? getNextGridItemId(app)

	const data = builddata(id)
	if (!app.subgrids) {
		app.subgrids = {}
	}

	// We only want to set subgrids when we are not moving
	if (!keepId) {
		for (let i = 0; i < (data.numberOfSubgrids ?? 0); i++) {
			app.subgrids[`${id}-${i}`] = []
		}
	}

	const key = focusedGrid
		? `${focusedGrid?.parentComponentId}-${focusedGrid?.subGridIndex ?? 0}`
		: undefined
	let grid = focusedGrid ? app.subgrids[key!] : app.grid

	const newItem = createNewGridItem(grid, id, data, columns)
	grid.push(newItem)

	return id
}

export function getAllSubgridsAndComponentIds(
	app: App,
	component: AppComponent
): [string[], string[]] {
	const subgrids: string[] = []
	let components: string[] = [component.id]
	if (component.type === 'tablecomponent') {
		components.push(...component.actionButtons?.map((x) => x.id))
	}
	if (app.subgrids && component.numberOfSubgrids) {
		for (let i = 0; i < component.numberOfSubgrids; i++) {
			const key = `${component.id}-${i}`
			const subgrid = app.subgrids[key]
			if (subgrid) {
				subgrids.push(key)
				for (const item of subgrid) {
					let [recSubgrids, recComponents] = getAllSubgridsAndComponentIds(app, item.data)
					subgrids.push(...recSubgrids)
					components.push(...recComponents)
				}
			}
		}
	}
	return [subgrids, components]
}

export function deleteGridItem(
	app: App,
	component: AppComponent,
	parent: string | undefined,
	shouldKeepSubGrid: boolean
): string[] {
	let [subgrids, components] = getAllSubgridsAndComponentIds(app, component)
	if (app.subgrids && !shouldKeepSubGrid) {
		subgrids.forEach((id) => {
			delete app.subgrids![id]
		})
	}
	if (parent) {
		app.subgrids &&
			(app.subgrids[parent] = app.subgrids[parent].filter((item) => item.id !== component?.id))
	} else {
		app.grid = app.grid.filter((item) => item.id !== component?.id)
	}
	return components
}

type AvailableSpace = {
	left: number
	right: number
	top: number
	bottom: number
}

export function findAvailableSpace(
	grid: GridItem[],
	gridItem: GridItem,
	editorBreakpoint: EditorBreakpoint,
	parentGridItem: GridItem | undefined = undefined
): AvailableSpace | undefined {
	if (gridItem) {
		const breakpoint = editorBreakpoint === 'sm' ? 3 : 12
		const maxHeight = parentGridItem ? parentGridItem[breakpoint].h - 1 : 16
		const maxWidth = 12

		const availableSpace = {
			left: 0,
			right: 0,
			top: 0,
			bottom: 0
		}

		const items = grid.map((item) => {
			return {
				id: item.id,
				x: item[breakpoint].x,
				y: item[breakpoint].y,
				w: item[breakpoint].w,
				h: item[breakpoint].h
			}
		})

		const item = items.find((item) => item.id === gridItem.id)

		if (!item) {
			return availableSpace
		}

		if (item.x > 0) {
			for (let x = item.x - 1; x >= 0; x--) {
				const itemToCheck = { ...item, x, w: 1 }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.left++
				}
			}
		}

		if (item.x + item.w < maxWidth) {
			for (let x = item.x + item.w; x < maxWidth; x++) {
				const itemToCheck = { ...item, x, w: 1 }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.right++
				}
			}
		}

		if (item.y > 0) {
			for (let y = item.y - 1; y >= 0; y--) {
				const itemToCheck = { ...item, h: 1, y }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.top++
				}
			}
		}

		if (item.y + item.h < maxHeight) {
			for (let y = item.y + item.h; y < maxHeight; y++) {
				const itemToCheck = { ...item, h: 1, y }
				const isItemInWay = items.some((item) => isOverlapping(item, itemToCheck))

				if (isItemInWay) {
					break
				} else {
					availableSpace.bottom++
				}
			}
		}

		return availableSpace
	}
}

function isOverlapping(item1: any, item2: any) {
	return (
		item1.x < item2.x + item2.w &&
		item1.x + item1.w > item2.x &&
		item1.y < item2.y + item2.h &&
		item1.y + item1.h > item2.y
	)
}

type Outputtable<Type> = {
	-readonly [Property in keyof Type]: Output<Type[Property]>
}

export function initOutput<I extends Record<string, any>>(
	world: World,
	id: string,
	init: I
): Outputtable<I> {
	world.initializedOutputs += 1
	if (!world) {
		return {} as any
	}
	return Object.fromEntries(
		Object.entries(init).map(([key, value]) => [key, world.newOutput(id, key, value)])
	) as Outputtable<I>
}

export function initConfig<
	T extends Record<
		string,
		| StaticAppInput
		| EvalAppInput
		| {
				type: 'oneOf'
				selected: string
				configuration: Record<string, Record<string, StaticAppInput | EvalAppInput>>
		  }
	>
>(
	r: T,
	configuration?: Record<
		string,
		| StaticAppInput
		| {
				type: 'oneOf'
				selected: string
				configuration: Record<string, Record<string, StaticAppInput | EvalAppInput>>
		  }
		| any
	>
): {
	[Property in keyof T]: T[Property] extends StaticAppInput
		? T[Property]['value'] | undefined
		: T[Property] extends { type: 'oneOf' }
		? {
				type: 'oneOf'
				selected: keyof T[Property]['configuration']
				configuration: {
					[Choice in keyof T[Property]['configuration']]: {
						[IT in keyof T[Property]['configuration'][Choice]]: T[Property]['configuration'][Choice][IT] extends StaticAppInput
							? T[Property]['configuration'][Choice][IT]['value'] | undefined
							: undefined
					}
				}
		  }
		: undefined
} {
	return JSON.parse(
		JSON.stringify(
			Object.fromEntries(
				Object.entries(r).map(([key, value]) =>
					value.type == 'static'
						? [
								key,
								configuration?.[key]?.type == 'static' ? configuration?.[key]?.['value'] : undefined
						  ]
						: value.type == 'oneOf'
						? [
								key,
								{
									selected: value.selected,
									type: 'oneOf',
									configuration: Object.fromEntries(
										Object.entries(value.configuration).map(([choice, config]) => [
											choice,
											initConfig(config, configuration?.[key]?.configuration?.[choice])
										])
									)
								}
						  ]
						: [key, undefined]
				)
			) as any
		)
	)
}

export function expandGriditem(
	grid: GridItem[],
	id: string,
	$breakpoint: EditorBreakpoint,
	parentGridItem: GridItem | undefined = undefined
) {
	const gridComponent = grid.find((item) => item.id === id)
	if (!gridComponent) return
	const availableSpace = findAvailableSpace(grid, gridComponent, $breakpoint, parentGridItem)
	if (!availableSpace) {
		return
	}

	const { left, right, top, bottom } = availableSpace
	const width = $breakpoint === 'sm' ? 3 : 12
	const item = gridComponent[width]

	item.x = item.x - left
	item.y = item.y - top
	item.w = item.w + left + right
	item.h = item.h + top + bottom
}

export function sortGridItemsPosition<T>(
	gridItems: FilledItem<T>[],
	width: number
): FilledItem<T>[] {
	return gridItems.sort((a: FilledItem<T>, b: FilledItem<T>) => {
		const aX = a[width].x
		const aY = a[width].y
		const bX = b[width].x
		const bY = b[width].y

		if (aY < bY) {
			return -1
		} else if (aY > bY) {
			return 1
		} else {
			if (aX < bX) {
				return -1
			} else if (aX > bX) {
				return 1
			} else {
				return 0
			}
		}
	})
}

export function connectInput(
	connectingInput: ConnectingInput,
	componentId: string,
	path: string
): ConnectingInput {
	if (connectingInput) {
		connectingInput = {
			opened: false,
			input: {
				connection: {
					componentId,
					path
				},
				type: 'connected'
			},
			hoveredComponent: undefined
		}
	}

	return connectingInput
}

export function recursivelyFilterKeyInJSON(
	json: object,
	search: string,
	extraSearch?: string | undefined
): object {
	if (json === null || json === undefined || typeof json != 'object') {
		return json
	}
	if (!search || search == '') {
		return json
	}
	let filteredJSON = {}
	Object.keys(json).forEach((key) => {
		if (
			key.toLowerCase().includes(search.toLowerCase()) ||
			extraSearch?.toLowerCase().includes(search.toLowerCase()) ||
			(typeof json[key] === 'string' && json[key].toLowerCase().includes(search.toLowerCase())) ||
			(typeof json[key] === 'number' &&
				json[key].toString().toLowerCase().includes(search.toLowerCase()))
		) {
			filteredJSON[key] = json[key]
		} else if (typeof json[key] === 'object') {
			const res = recursivelyFilterKeyInJSON(json[key], search, extraSearch)

			if (Object.keys(res ?? {}).length !== 0) {
				filteredJSON[key] = res
			}
		}
	})
	return filteredJSON
}

export function clearErrorByComponentId(
	id: string,
	errorByComponent: Record<
		string,
		{
			error: string
			componentId: string
		}
	>
) {
	return Object.entries(errorByComponent).reduce((acc, [key, value]) => {
		if (value.componentId !== id) {
			acc[key] = value
		}
		return acc
	}, {})
}

export function clearJobsByComponentId(
	id: string,
	jobs: {
		job: string
		component: string
	}[]
) {
	return jobs.filter((job) => job.component !== id)
}

// Returns the error message for the latest job for a component if an error occurred, otherwise undefined
export function getErrorFromLatestResult(
	id: string,
	errorByComponent: Record<
		string, // job id
		{
			error: string
			componentId: string
		}
	>,
	jobs: {
		job: string
		component: string
	}[]
) {
	// find last jobId for component id
	const lastJob = jobs.find((job) => job.component === id)

	if (lastJob?.job && errorByComponent[lastJob.job]) {
		return errorByComponent[lastJob.job].error
	} else {
		return undefined
	}
}
