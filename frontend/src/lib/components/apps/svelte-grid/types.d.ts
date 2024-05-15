export interface Size {
	w: number
	h: number
}

export interface Positon {
	x: number
	y: number
}

interface ItemLayout extends Size, Positon {
	fixed: boolean
	fullHeight: boolean
}

export type FilledItem<T> = { [width: number]: Required<ItemLayout>; data: T; id: string }
