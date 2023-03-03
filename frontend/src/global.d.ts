/// <reference types="@sveltejs/kit" />

declare type Item = import('svelte-dnd-action').Item
declare type DndEvent<ItemType = Item> = import('svelte-dnd-action').DndEvent<ItemType>
declare namespace svelte.JSX {
	interface HTMLAttributes<T> {
		onconsider?: (event: CustomEvent<DndEvent<ItemType>> & { target: EventTarget & T }) => void
		onfinalize?: (event: CustomEvent<DndEvent<ItemType>> & { target: EventTarget & T }) => void
	}
}

declare module '@windmill-labs/svelte-grid' {
	import type { SvelteComponentTyped } from 'svelte'

	export interface Size {
		w: number
		h: number
	}

	export interface Positon {
		x: number
		y: number
	}

	interface ItemLayout extends Size, Positon {
		fixed?: boolean
		resizable?: boolean
		draggable?: boolean
		customDragger?: boolean
		customResizer?: boolean
		min?: Size
		max?: Size
	}

	export type Item<T> = T & { [width: number]: ItemLayout; data: any }
	export type FilledItem<T> = T & { [width: number]: Required<ItemLayout>; data: any }

	export interface Props<T> {
		fillSpace?: boolean
		items: FilledItem<T>[]
		rowHeight: number
		cols: [number, number][]
		gap?: [number, number]
		fastStart?: boolean
		throttleUpdate?: number
		throttleResize?: number
		onTopId?: string
		scroller?: undefined
		sensor?: number
		parentWidth?: number
	}

	export interface Slots<T> {
		default: { item: ItemLayout; dataItem: Item<T> }
	}

	export default class Grid<T = {}> extends SvelteComponentTyped<
		Props<T>,
		{
			pointerup: CustomEvent<{ id: string }>
			mount: CustomEvent<>
		},
		Slots<T>
	> { }
}

declare module '@windmill-labs/svelte-grid/build/helper/index.mjs' {
	import { ItemLayout } from '@windmill-labs/svelte-grid'

	const x: {
		normalize(items: any[], col: any): unknown[]
		adjust(items: any[], col: any): unknown[]
		findSpace(item: any, items: any, cols: any): unknown

		item<T>(obj: ItemLayout): Required<ItemLayout>
	}

	export default x
}
