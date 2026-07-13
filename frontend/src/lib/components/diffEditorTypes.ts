import type { ButtonType } from './common'

export interface ButtonProp {
	text: string
	color?: ButtonType.Color
	onClick: () => void
}

// Below this editor width, side-by-side is too cramped and the diff falls back
// to the unified/inline view. Shared so consumers (e.g. WorkspaceDiffDrawer's
// view toggle) can mirror the same threshold instead of duplicating the number.
export const SIDE_BY_SIDE_MIN_WIDTH = 700
