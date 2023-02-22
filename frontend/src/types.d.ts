import type { AppComponent } from '$lib/components/apps/types'

declare module 'simple-svelte-autocomplete'

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

	export type Item<T> = T & { [width: number]: ItemLayout }
	export type FilledItem<T> = T & { [width: number]: Required<ItemLayout> }

	export interface Props<T> {
		fillSpace?: boolean
		items: FilledItem<T>[]
		rowHeight: number
		cols: [number, number][]
		gap?: [number, number]
		fastStart?: boolean
		throttleUpdate?: number
		throttleResize?: number

		scroller?: undefined
		sensor?: number
	}

	export interface Slots<T> {
		default: { item: ItemLayout; dataItem: Item<T> & { data: AppComponent } }
	}

	export default class Grid<T = {}> extends SvelteComponentTyped<Props<T>, {}, Slots<T>> {}
}

declare module '@windmill-labs/svelte-grid/build/helper/index.mjs' {
	import { ItemLayout } from '@windmill-labs/svelte-grid'

	const x: {
		normalize(items: any[], col: any): unknown[]
		adjust(items: any[], col: any): Item<unknown>[]
		findSpace(item: any, items: any, cols: any): unknown[]

		item<T>(obj: ItemLayout): Required<ItemLayout>
	}

	export default x
}
