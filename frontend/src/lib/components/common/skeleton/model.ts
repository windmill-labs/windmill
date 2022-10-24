export const HEIGHT_UNIT = 16 as const

export type SkeletonLayout = (SkeletonGap | SkeletonRow)[]

/** **Gap**: number of height units. *(1 unit = 16px)* */
export type SkeletonGap = number

export type SkeletonRow = SkeletonElement[] | ISkeletonRow

export interface ISkeletonRow {
	/** Number of elements in the row.
	 * All elements will have the same width and height of the row. */
	elements: number
	/** **Height**: number of height units. *(1 unit = 16px)* */
	h: number
}

export type SkeletonElement = number | ISkeletonElement

export interface ISkeletonElement {
	/** **Height**: number of height units. *(1 unit = 16px)* */
	h: number
	/** **Width**: percentage of the row width. *(80 means 80%)* */
	w: number
	/** **Min Width**: minimum width in pixels. */
	minW?: number
}
