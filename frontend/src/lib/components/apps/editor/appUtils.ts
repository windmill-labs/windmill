import type {
	App,
	BaseAppComponent,
	ComponentCustomCSS,
	ConnectingInput,
	EditorBreakpoint,
	FocusedGrid,
	GeneralAppInput,
	GridItem,
	RichConfiguration
} from '../types'
import {
	ccomponents,
	components,
	getRecommendedDimensionsByComponent,
	presets,
	processDimension,
	type AppComponent,
	type BaseComponent,
	type InitialAppComponent,
	type TypedComponent
} from './component'
import { gridColumns } from '../gridUtils'
import { processSubcomponents } from '../utils'
import type { Output, World } from '../rx'
import type { FilledItem, Size } from '../svelte-grid/types'
import type {
	StaticAppInput,
	EvalAppInput,
	EvalV2AppInput,
	InputConnectionEval,
	StaticAppInputOnDemand
} from '../inputType'
import { get, type Writable } from 'svelte/store'
import { deepMergeWithPriority } from '$lib/utils'
import { sendUserToast } from '$lib/toast'
import { getNextId } from '$lib/components/flows/idUtils'
import { enterpriseLicense } from '$lib/stores'
import gridHelp from '../svelte-grid/utils/helper'
import { DEFAULT_THEME } from './componentsPanel/themeUtils'
import { allItems, findGridItem, findGridItemById } from './appUtilsCore'

type GridItemLocation =
	| {
			type: 'grid'
			gridItemIndex: number
	  }
	| {
			type: 'subgrid'
			subgridItemIndex: number
			subgridKey: string
	  }
export interface GridItemWithLocation {
	location: GridItemLocation
	item: GridItem
	parent: string | undefined
}

export function allItemsWithLocation(
	grid: GridItem[],
	subgrids: Record<string, GridItem[]> | undefined
): GridItemWithLocation[] {
	const gridItems: GridItemWithLocation[] = grid.map((x, i) => ({
		location: {
			type: 'grid',
			gridItemIndex: i
		},
		item: x,
		parent: undefined
	}))
	if (subgrids) {
		for (const key of Object.keys(subgrids)) {
			gridItems.push(
				...subgrids[key].map((x, i) => ({
					location: {
						type: 'subgrid' as const,
						subgridItemIndex: i,
						subgridKey: key
					},
					item: x,
					parent: key
				}))
			)
		}
	}
	return gridItems
}

export function findGridItemWithLocation(
	app: App,
	id: string | undefined
): GridItemWithLocation | undefined {
	if (!id) return undefined
	if (app?.grid) {
		const gridItemIndex = app.grid.findIndex((x) => x.data?.id === id)
		if (gridItemIndex > -1) {
			return {
				location: {
					type: 'grid',
					gridItemIndex: gridItemIndex
				},
				item: app.grid[gridItemIndex],
				parent: undefined
			}
		}
	}

	if (app?.subgrids) {
		for (const key of Object.keys(app.subgrids ?? {})) {
			const subGridItemIndex = app.subgrids[key].findIndex((x) => x.data?.id === id)
			if (subGridItemIndex > -1) {
				return {
					location: {
						subgridItemIndex: subGridItemIndex,
						subgridKey: key,
						type: 'subgrid'
					},
					item: app.subgrids[key][subGridItemIndex],
					parent: key
				}
			}
		}
	}

	return undefined
}

export function findComponentSettings(app: App, id: string | undefined) {
	if (!id) return undefined
	if (app?.grid) {
		const gridItem = app.grid.find((x) => x.data?.id === id)
		if (gridItem) {
			return { item: gridItem, parent: undefined }
		}
	}

	if (app?.subgrids) {
		for (const key of Object.keys(app.subgrids ?? {})) {
			const gridItem = app.subgrids[key].find((x) => x.data?.id === id)
			if (gridItem) {
				return { item: gridItem, parent: key }
			}
		}
	}

	return undefined
}
export function dfs(
	grid: GridItem[],
	id: string,
	subgrids: Record<string, GridItem[]>
): string[] | undefined {
	if (!grid) {
		return undefined
	}
	for (const item of grid) {
		if (item.id === id) {
			return [id]
		} else if (
			item.data.type == 'tablecomponent' &&
			item.data.actionButtons.find((x) => id == x.id)
		) {
			return [item.id, id]
		} else if (item.data.type == 'menucomponent' && item.data.menuItems.find((x) => id == x.id)) {
			return [item.id, id]
		} else if (
			(item.data.type == 'aggridcomponent' ||
				item.data.type == 'aggridcomponentee' ||
				item.data.type == 'dbexplorercomponent' ||
				item.data.type == 'aggridinfinitecomponent' ||
				item.data.type == 'aggridinfinitecomponentee') &&
			item.data.actions?.find((x) => id == x.id)
		) {
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
	;(document?.activeElement as HTMLElement)?.blur()
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

export function connectOutput(
	connectingInput: Writable<ConnectingInput>,
	typ: TypedComponent['type'],
	id: string,
	spath: string
) {
	if (get(connectingInput).opened) {
		let splitted = spath?.split('.')
		let componentId = typ == 'containercomponent' ? splitted?.[0] : id
		let path = typ == 'containercomponent' ? splitted?.[1] : spath
		connectingInput.set(connectInput(get(connectingInput), componentId, path, typ))
	}
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
			if (
				(item.data.type === 'aggridcomponent' ||
					item.data.type === 'aggridcomponentee' ||
					item.data.type === 'dbexplorercomponent' ||
					item.data.type === 'aggridinfinitecomponent' ||
					item.data.type === 'aggridinfinitecomponentee') &&
				Array.isArray(item.data.actions)
			) {
				subIds.push(...item.data.actions?.map((x) => x.id))
			}
			if (item.data.type === 'menucomponent') {
				subIds.push(...item.data.menuItems?.map((x) => x.id))
			}
		}
		return subIds
	}
	return getAllSubgridsAndComponentIds(app, item?.data)[1]
}

export function getNextGridItemId(app: App): string {
	const allIds = allItems(app.grid, app.subgrids).flatMap((x) => {
		if (x.data.type === 'tablecomponent') {
			return [x.id, ...x.data.actionButtons.map((x) => x.id)]
		} else if (
			(x.data.type === 'aggridcomponent' ||
				x.data.type === 'aggridcomponentee' ||
				x.data.type === 'dbexplorercomponent' ||
				x.data.type === 'aggridinfinitecomponent' ||
				x.data.type === 'aggridinfinitecomponentee') &&
			Array.isArray(x.data.actions)
		) {
			return [x.id, ...x.data.actions.map((x) => x.id)]
		} else if (x.data.type === 'menucomponent') {
			return [x.id, ...x.data.menuItems.map((x) => x.id)]
		} else {
			return [x.id]
		}
	})
	return getNextId(allIds)
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
	columns?: Record<number, any>,
	initialPosition: { x: number; y: number } = { x: 0, y: 0 },
	recOverride?: Record<number, Size>,
	fixed?: boolean,
	shouldNotFindSpace?: boolean
): GridItem {
	const newComponent = {
		fixed: fixed ?? false,
		x: initialPosition.x,
		y: initialPosition.y,
		fullHeight: false
	}

	let newData: AppComponent = JSON.parse(JSON.stringify(data))
	newData.id = id

	const newItem: GridItem = {
		data: newData,
		id: id
	}

	gridColumns.forEach((column) => {
		if (!columns) {
			const rec = recOverride?.[column] ?? getRecommendedDimensionsByComponent(newData.type, column)

			newItem[column] = {
				...newComponent,
				w: rec.w,
				h: rec.h
			}
		} else {
			newItem[column] = {
				...columns[column],
				x: initialPosition.x,
				y: initialPosition.y
			}
		}

		let shouldComputePosition: boolean = false

		// Fallback to avoid component disapearing
		if (initialPosition.x === undefined || initialPosition.y === undefined) {
			newItem[column].x = 0
			newItem[column].y = 0
			shouldComputePosition = true
		}

		// Either the final position is controlled using initialPosition or the position is computed because the component positions are wrong
		if (!shouldNotFindSpace || shouldComputePosition) {
			const position = gridHelp.findSpace(newItem, grid, column) as { x: number; y: number }

			newItem[column] = { ...newItem[column], ...position }
		}
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

function cleanseValue(
	key: string,
	value: {
		type: 'eval' | 'static' | 'evalv2'
		value?: any
		expr?: string
		connections?: InputConnectionEval[]
	}
) {
	if (!value) {
		return [key, undefined]
	}
	if (value.type === 'static') {
		return [key, { type: value.type, value: value.value }]
	} else if (value.type === 'eval') {
		return [key, { type: value.type, expr: value.expr }]
	} else {
		return [key, { type: value.type, expr: value.expr, connections: value.connections }]
	}
}

export function cleanseOneOfConfiguration(
	configuration: Record<
		string,
		Record<string, GeneralAppInput & (StaticAppInput | EvalAppInput | EvalV2AppInput)>
	>
) {
	return Object.fromEntries(
		Object.entries(configuration).map(([key, val]) => [
			key,
			Object.fromEntries(Object.entries(val).map(([key, val]) => cleanseValue(key, val)))
		])
	)
}

export function appComponentFromType<T extends keyof typeof components>(
	type: T,
	overrideConfiguration?: Partial<InitialAppComponent['configuration']>,
	extra?: any,
	override?: {
		customCss?: ComponentCustomCSS<T>
		verticalAlignment?: 'top' | 'center' | 'bottom'
		horizontalAlignment?: 'left' | 'center' | 'right'
		componnetInput?: Partial<InitialAppComponent['componentInput']>
	}
): (id: string) => BaseAppComponent & BaseComponent<T> {
	return (id: string) => {
		const init = JSON.parse(JSON.stringify(ccomponents[type].initialData)) as InitialAppComponent

		const configuration = Object.fromEntries(
			Object.entries(init.configuration).map(([key, value]) => {
				if (value.type != 'oneOf') {
					return cleanseValue(key, value)
				} else {
					return [
						key,
						{
							type: value.type,
							selected: value.selected,
							configuration: cleanseOneOfConfiguration(value.configuration)
						}
					]
				}
			})
		)

		return {
			type,
			//TODO remove tooltip from there
			configuration: deepMergeWithPriority(configuration, overrideConfiguration ?? {}),
			componentInput: override?.componnetInput ?? init.componentInput,
			panes: init.panes,
			tabs: init.tabs,
			conditions: init.conditions,
			nodes: init.nodes,
			customCss: deepMergeWithPriority(
				ccomponents[type].customCss as any,
				override?.customCss ?? {}
			),
			recomputeIds: init.recomputeIds ? [] : undefined,
			actionButtons: init.actionButtons ? [] : undefined,
			actions: undefined,
			menuItems: init.menuItems ? [] : undefined,
			numberOfSubgrids: init.numberOfSubgrids,
			horizontalAlignment: override?.horizontalAlignment ?? init.horizontalAlignment,
			verticalAlignment: override?.verticalAlignment ?? init.verticalAlignment,
			id,
			datasets:
				type === 'plotlycomponentv2'
					? createPlotlyComponentDataset()
					: type === 'chartjscomponentv2'
						? createChartjsComponentDataset()
						: undefined,
			xData:
				type === 'plotlycomponentv2' || type === 'chartjscomponentv2'
					? {
							type: 'evalv2',
							fieldType: 'array',
							expr: '[1, 2, 3, 4]',
							connections: []
						}
					: undefined,
			...(extra ?? {})
		}
	}
}

export function createChartjsComponentDataset(): RichConfiguration {
	return {
		type: 'static',
		fieldType: 'array',
		subFieldType: 'chartjs',
		value: [
			{
				value: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'number',
					value: [25, 25, 50]
				},
				name: 'Dataset 1'
			}
		]
	}
}

export function createPlotlyComponentDataset(): RichConfiguration {
	return {
		type: 'static',
		fieldType: 'array',
		subFieldType: 'plotly',
		value: [
			{
				value: {
					type: 'static',
					fieldType: 'array',
					subFieldType: 'number',
					value: [1, 2, 3, 4]
				},
				name: 'Dataset 1',
				aggregation_method: 'sum',
				type: 'bar',
				tooltip: '',
				color: '#C8A2C8'
			}
		]
	}
}
export function insertNewGridItem(
	app: App,
	builddata: (id: string) => AppComponent,
	focusedGrid: FocusedGrid | undefined,
	columns?: Record<string, any>,
	keepId?: string,
	initialPosition: { x: number; y: number } = { x: 0, y: 0 },
	recOverride?: Record<number, Size>,
	keepSubgrids?: boolean,
	fixed?: boolean,
	shouldNotFindSpace?: boolean
): string {
	const id = keepId ?? getNextGridItemId(app)

	const data = builddata(id)

	if (data.type == 'aggridcomponentee' && !get(enterpriseLicense)) {
		sendUserToast('AgGrid Enterprise Edition require Windmill Enterprise Edition', true)
		throw Error('AgGrid Enterprise Edition require Windmill Enterprise Edition')
	}
	if (!app.subgrids) {
		app.subgrids = {}
	}

	// We only want to set subgrids when we are not moving
	if (!keepId || keepSubgrids) {
		for (let i = 0; i < (data.numberOfSubgrids ?? 0); i++) {
			app.subgrids[`${id}-${i}`] = []
		}
	}

	const key = focusedGrid
		? `${focusedGrid?.parentComponentId}-${focusedGrid?.subGridIndex ?? 0}`
		: undefined

	if (key && app.subgrids[key] === undefined) {
		let parent = findGridItemById(app.grid, app.subgrids, key)?.data
		let subgrids = parent?.numberOfSubgrids
		if (subgrids === undefined) {
			throw Error(
				`Invalid subgrid selected, the parent has no subgrids: ${key}, parent: ${JSON.stringify(
					parent
				)}`
			)
		}
		if (
			focusedGrid?.subGridIndex &&
			(focusedGrid?.subGridIndex < 0 || focusedGrid?.subGridIndex >= subgrids)
		) {
			throw Error(`Invalid subgrid selected: ${key}, max subgrids: ${subgrids}`)
		}
		// If ever the subgrid is undefined, we want to make sure it is defined
		app.subgrids[key] = []
	}

	let grid = focusedGrid ? app.subgrids[key!] : app.grid

	const newItem = createNewGridItem(
		grid,
		id,
		data,
		columns,
		initialPosition,
		recOverride,
		fixed,
		shouldNotFindSpace
	)
	grid.push(newItem)
	return id
}

export function copyComponent(
	app: App,
	item: GridItem,
	parentGrid: FocusedGrid | undefined,
	subgrids: Record<string, GridItem[]>,
	alreadyVisited: string[]
) {
	if (alreadyVisited.includes(item.id)) {
		return
	} else {
		alreadyVisited.push(item.id)
	}
	const newItem = insertNewGridItem(
		app,
		(id) => {
			let newComponent = { ...item.data, id }
			processSubcomponents(newComponent, (c) => {
				c.id = c.id.replace(item.id + '_', id + '_')
			})
			return newComponent
		},
		parentGrid,
		Object.fromEntries(gridColumns.map((column) => [column, item[column]]))
	)

	for (let i = 0; i < (item?.data?.numberOfSubgrids ?? 0); i++) {
		subgrids[`${item.id}-${i}`].forEach((subgridItem) => {
			copyComponent(
				app,
				subgridItem,
				{ parentComponentId: newItem, subGridIndex: i },
				subgrids,
				alreadyVisited
			)
		})
	}

	return newItem
}

export function getAllSubgridsAndComponentIds(
	app: App,
	component: AppComponent
): [string[], string[]] {
	const subgrids: string[] = []
	let components: string[] = [component.id]
	processSubcomponents(component, (c) => {
		components.push(c.id)
	})

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

export function getAllGridItems(app: App): GridItem[] {
	return app.grid
		.concat(Object.values(app.subgrids ?? {}).flat())
		.map((x) => {
			let r: GridItem[] = []
			processSubcomponents(x.data, (c) => {
				r.push({ data: c, id: c.id })
			})
			return [x, ...r]
		})
		.flat()
}

export function deleteGridItem(
	app: App,
	component: AppComponent,
	parent: string | undefined
): string[] {
	let [subgrids, components] = getAllSubgridsAndComponentIds(app, component)
	if (app.subgrids) {
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
	if (!world) {
		return {} as any
	}
	return Object.fromEntries(
		Object.entries(init).map(([key, value]) => [key, world.newOutput(id, key, value)])
	) as Outputtable<I>
}

export type InitConfig<
	T extends Record<
		string,
		| StaticAppInput
		| EvalAppInput
		| EvalV2AppInput
		| {
				type: 'oneOf'
				selected: string
				configuration: Record<
					string,
					Record<string, StaticAppInput | EvalAppInput | EvalV2AppInput>
				>
		  }
	>
> = {
	[Property in keyof T]: T[Property] extends StaticAppInput
		? T[Property]['value'] | undefined
		: T[Property] extends { type: 'oneOf' }
			? {
					type: 'oneOf'
					selected: keyof T[Property]['configuration']
					configuration: {
						[Choice in keyof T[Property]['configuration']]: {
							[IT in keyof T[Property]['configuration'][Choice]]: T[Property]['configuration'][Choice][IT] extends StaticAppInput
								? T[Property]['configuration'][Choice][IT] extends StaticAppInputOnDemand
									? () => Promise<T[Property]['configuration'][Choice][IT]['value'] | undefined>
									: T[Property]['configuration'][Choice][IT]['value'] | undefined
								: undefined
						}
					}
				}
			: undefined
}

export function initConfig<
	T extends Record<
		string,
		| StaticAppInput
		| EvalAppInput
		| EvalV2AppInput
		| {
				type: 'oneOf'
				selected: string
				configuration: Record<
					string,
					Record<string, StaticAppInput | EvalAppInput | EvalV2AppInput>
				>
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
				configuration: Record<
					string,
					Record<string, StaticAppInput | EvalAppInput | EvalV2AppInput | boolean>
				>
		  }
		| any
	>
): InitConfig<T> {
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
											Object.entries(value.configuration).map(([choice, config]) => {
												const conf = initConfig(
													config,
													configuration?.[key]?.configuration?.[choice]
												)
												Object.entries(config).forEach(([innerKey, innerValue]) => {
													if (innerValue.type === 'static' && !(innerKey in conf)) {
														conf[innerKey] = innerValue.value
													}
												})
												return [choice, conf]
											})
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
	path: string,
	componentType?: TypedComponent['type']
): ConnectingInput {
	if (connectingInput) {
		if (connectingInput.onConnect) {
			connectingInput.onConnect({ componentId, path })
			sendUserToast(`Connected to ${componentId}.${path}`, false)
		}

		connectingInput = {
			opened: false,
			input: {
				connection: {
					componentId,
					path,
					componentType
				},
				type: 'connected'
			},
			hoveredComponent: undefined,
			onConnect: undefined
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

export const ROW_HEIGHT = 36
export const ROW_GAP_Y = 2
export const ROW_GAP_X = 4

export function maxHeight(
	grid: GridItem[],
	windowHeight: number,
	breakpoint: EditorBreakpoint = 'lg'
) {
	const totalRowHeight = ROW_HEIGHT + ROW_GAP_Y
	let maxRows = Math.floor((windowHeight - ROW_GAP_Y) / totalRowHeight)

	if (!grid.length) {
		return maxRows
	}

	const breakpointKey = breakpoint === 'sm' ? 3 : 12
	const maxRowPerGrid = grid.reduce((max, item) => {
		const y = item[breakpointKey].y
		const h = item[breakpointKey].h
		return Math.max(max, y + h)
	}, 0)

	return Math.max(maxRowPerGrid, maxRows)
}

export function isTableAction(id: string, app: App): boolean {
	const [tableId, actionId] = id.split('_')

	if (!tableId || !actionId) {
		return false
	}

	const table = findGridItem(app, tableId)
	if (
		!table ||
		(table.data.type !== 'tablecomponent' &&
			table.data.type !== 'aggridcomponent' &&
			table.data.type !== 'aggridcomponentee' &&
			table.data.type !== 'dbexplorercomponent' &&
			table.data.type !== 'aggridinfinitecomponent' &&
			table.data.type !== 'aggridinfinitecomponentee')
	) {
		return false
	}
	return true
}

export function setUpTopBarComponentContent(id: string, app: App) {
	insertNewGridItem(
		app,
		appComponentFromType(
			'textcomponent',

			{
				disableNoText: {
					value: true,
					type: 'static',
					fieldType: 'boolean'
				},
				tooltip: {
					type: 'evalv2',

					fieldType: 'text',
					expr: '`Author: ${ctx.author}`',
					connections: [
						{
							componentId: 'ctx',
							id: 'author'
						}
					]
				}
			},
			undefined,
			{
				customCss: {
					text: {
						class: 'text-xl font-semibold whitespace-nowrap truncate' as any,
						style: ''
					}
				},
				verticalAlignment: 'center',
				componnetInput: {
					type: 'templatev2',
					fieldType: 'template',
					eval: '${ctx.summary}',
					connections: [
						{
							id: 'summary',
							componentId: 'ctx'
						}
					] as InputConnectionEval[]
				}
			}
		) as (id: string) => AppComponent,
		{
			parentComponentId: id,
			subGridIndex: 0
		},
		undefined,
		'title',
		undefined,
		{
			3: {
				w: 6,
				h: 1
			},
			12: {
				w: 6,
				h: 1
			}
		}
	)

	insertNewGridItem(
		app,
		appComponentFromType('recomputeallcomponent', undefined, undefined, {
			horizontalAlignment: 'right'
		}) as (id: string) => AppComponent,
		{
			parentComponentId: id,
			subGridIndex: 0
		},
		undefined,
		'recomputeall',
		undefined,
		{
			3: {
				w: 3,
				h: 1
			},
			12: {
				w: 6,
				h: 1
			}
		}
	)
}

export function isContainer(type: string): boolean {
	return (
		type === 'containercomponent' ||
		type === 'tabscomponent' ||
		type === 'verticalsplitpanescomponent' ||
		type === 'horizontalsplitpanescomponent' ||
		type === 'steppercomponent' ||
		type === 'listcomponent' ||
		type === 'carousellistcomponent' ||
		type === 'decisiontreecomponent'
	)
}

export function subGridIndexKey(type: string | undefined, id: string, world: World): number {
	switch (type) {
		case 'containercomponent':
		case 'listcomponent':
			return 0
		case 'verticalsplitpanescomponent':
		case 'horizontalsplitpanescomponent':
			return (world?.outputsById?.[id]?.selectedPaneIndex?.peak() as number) ?? 0
		case 'tabscomponent': {
			return (world?.outputsById?.[id]?.selectedTabIndex?.peak() as number) ?? 0
		}
		case 'steppercomponent': {
			return (world?.outputsById?.[id]?.currentStepIndex?.peak() as number) ?? 0
		}
		case 'carousellistcomponent': {
			return (world?.outputsById?.[id]?.currentIndex?.peak() as number) ?? 0
		}
		case 'decisiontreecomponent': {
			return (world?.outputsById?.[id]?.currentNodeIndex?.peak() as number) ?? 0
		}
	}

	return 0
}

export function computePosition(
	clientX: number,
	clientY: number,
	xPerPx: number,
	yPerPx: number,
	overlapped?: string,
	element?: HTMLElement
) {
	const overlappedElement = overlapped
		? document.getElementById(`component-${overlapped}`)
		: document.getElementById('root-grid')

	const xRelativeToElement = element ? clientX - element.getBoundingClientRect().left : 0
	const yRelativeToElement = element ? clientY - element.getBoundingClientRect().top : 0

	const xRelativeToOverlappedElement = overlappedElement
		? clientX - overlappedElement.getBoundingClientRect().left - xRelativeToElement
		: 0
	const yRelativeToOverlappedElement = overlappedElement
		? clientY - overlappedElement.getBoundingClientRect().top - yRelativeToElement
		: 0

	const gridX = Math.max(Math.round(xRelativeToOverlappedElement / xPerPx) ?? 0, 0)
	const gridY = Math.max(Math.round(yRelativeToOverlappedElement / yPerPx) ?? 0, 0)

	return {
		x: gridX,
		y: gridY
	}
}

export function getDeltaYByComponent(type: string) {
	switch (type) {
		case 'steppercomponent': {
			return '36px + 0.5rem'
		}
		case 'tabscomponent': {
			return '32px'
		}
		default:
			return '0px'
	}
}

export function getDeltaXByComponent(type: string) {
	switch (type) {
		case 'steppercomponent': {
			return '0.5rem'
		}
		case 'tabscomponent': {
			return '0px'
		}
		default:
			return '0px'
	}
}

export type GridShadow = {
	x: number
	y: number
	xPerPx: number
	yPerPx: number
	w: number
	h: number
}

export function areShadowsTheSame(
	shadow1: GridShadow | undefined,
	shadow2: GridShadow | undefined
) {
	if (!shadow1 || !shadow2) {
		return false
	}

	return (
		shadow1.x === shadow2.x &&
		shadow1.y === shadow2.y &&
		shadow1.xPerPx === shadow2.xPerPx &&
		shadow1.yPerPx === shadow2.yPerPx &&
		shadow1.w === shadow2.w &&
		shadow1.h === shadow2.h
	)
}

export function animateTo(start: number, end: number, onUpdate: (newValue: number) => void) {
	const duration = 400
	const startTime = performance.now()

	function animate(time: number) {
		const elapsed = time - startTime
		const progress = Math.min(elapsed / duration, 1)
		const currentValue = start + (end - start) * easeInOut(progress)
		onUpdate(currentValue)
		if (progress < 1) {
			requestAnimationFrame(animate)
		}
	}

	requestAnimationFrame(animate)
}

function easeInOut(t: number) {
	return t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t
}

export function emptyApp(): App {
	let value: App = {
		grid: [],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		theme: {
			type: 'path',
			path: DEFAULT_THEME
		}
	}
	const preset = presets['topbarcomponent']

	const id = insertNewGridItem(
		value,
		appComponentFromType(preset.targetComponent, preset.configuration, undefined, {
			customCss: {
				container: {
					class: '!p-0' as any,
					style: ''
				}
			}
		}) as (id: string) => AppComponent,
		undefined,
		undefined,
		'topbar',
		{ x: 0, y: 0 },
		{
			3: processDimension(preset.dims, 3),
			12: processDimension(preset.dims, 12)
		},
		true,
		true
	)

	setUpTopBarComponentContent(id, value)

	value.hideLegacyTopBar = true
	value.mobileViewOnSmallerScreens = false
	return value
}
