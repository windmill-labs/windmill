export type Option = string | number | ObjectOption

// single CSS string or an object with keys 'option' and 'selected', each a string,
// which only apply to the dropdown list and list of selected options, respectively
export type OptionStyle = string | { option: string; selected: string }

export type ObjectOption = {
	label: string | number // user-displayed text
	value?: unknown // associated value, can be anything incl. objects (defaults to label if undefined)
	title?: string // on-hover tooltip
	disabled?: boolean // make this option unselectable
	preselected?: boolean // make this option selected on page load (before any user interaction)
	disabledTitle?: string // override the default disabledTitle = 'This option is disabled'
	selectedTitle?: string // tooltip to display when this option is selected and hovered
	style?: OptionStyle
	[key: string]: unknown // allow any other keys users might want
}

export type DispatchEvents<T = Option> = {
	add: { option: T }
	create: { option: T }
	remove: { option: T }
	removeAll: { options: T[] }
	change: {
		option?: T
		options?: T[]
		type: 'add' | 'remove' | 'removeAll'
	}
	open: { event: Event }
	close: { event: Event }
}

export type MultiSelectEvents = {
	[key in keyof DispatchEvents]: CustomEvent<DispatchEvents[key]>
} & {
	blur: FocusEvent
	click: MouseEvent
	focus: FocusEvent
	keydown: KeyboardEvent
	keyup: KeyboardEvent
	mouseenter: MouseEvent
	mouseleave: MouseEvent
	touchcancel: TouchEvent
	touchend: TouchEvent
	touchmove: TouchEvent
	touchstart: TouchEvent
}
