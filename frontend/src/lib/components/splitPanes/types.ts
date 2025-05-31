export type Pane = {
	size: number | undefined
	active: boolean
}

export type PanesLayout = Pane[]

export type SplitPanesContext = {
	sizes: (index: number) => number | undefined
	setActivePane: (index: number) => void
}
