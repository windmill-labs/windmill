import type { ButtonType } from './common'

export interface ButtonProp {
	text: string
	color?: ButtonType.Color
	onClick: () => void
}
