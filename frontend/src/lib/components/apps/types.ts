import type { Schema } from '$lib/common'
import type { Preview } from '$lib/gen'
import type { FilledItem } from 'svelte-grid'
import type { Writable } from 'svelte/store'
import type {
	AppInput,
	ConnectedAppInput,
	ConnectedInput,
	StaticAppInput,
	UserAppInput
} from './inputType'
import type { World } from './rx'

type BaseComponent<T extends string> = {
	type: T
}

export type TextComponent = BaseComponent<'textcomponent'>
export type TextInputComponent = BaseComponent<'textinputcomponent'>
export type PasswordInputComponent = BaseComponent<'passwordinputcomponent'>
export type DateInputComponent = BaseComponent<'dateinputcomponent'>
export type NumberInputComponent = BaseComponent<'numberinputcomponent'>
export type ButtonComponent = BaseComponent<'buttoncomponent'> & {
	recomputeIds: string[] | undefined
}

export type FormComponent = BaseComponent<'formcomponent'> & {
	recomputeIds: string[] | undefined
}

export type RunFormComponent = BaseComponent<'runformcomponent'>
export type BarChartComponent = BaseComponent<'barchartcomponent'>
export type PieChartComponent = BaseComponent<'piechartcomponent'>
export type TableComponent = BaseComponent<'tablecomponent'> & {
	actionButtons: (BaseAppComponent & ButtonComponent)[]
}

export type DisplayComponent = BaseComponent<'displaycomponent'>
export type ImageComponent = BaseComponent<'imagecomponent'>
export type InputComponent = BaseComponent<'inputcomponent'>
export type SelectComponent = BaseComponent<'selectcomponent'>
export type CheckboxComponent = BaseComponent<'checkboxcomponent'>
export type RadioComponent = BaseComponent<'radiocomponent'>

export type HorizontalAlignment = 'left' | 'center' | 'right'
export type VerticalAlignment = 'top' | 'center' | 'bottom'

export type Aligned = {
	horizontalAlignment: HorizontalAlignment
	verticalAlignment: VerticalAlignment
}

export interface BaseAppComponent extends Partial<Aligned> {
	id: ComponentID
	componentInput: AppInput | undefined
	configuration: Record<string, StaticAppInput | ConnectedAppInput | UserAppInput>
	card: boolean | undefined
	/**
	 * If `true` then the wrapper will allow items to flow outside of it's borders.
	 *
	 * *For example when the component has a popup like `Select`*
	 */
	softWrap?: boolean
	// TODO: add min/max width/height
}

export type AppComponent = BaseAppComponent &
	(
		| RunFormComponent
		| DisplayComponent
		| TextInputComponent
		| PasswordInputComponent
		| DateInputComponent
		| NumberInputComponent
		| BarChartComponent
		| TableComponent
		| TextComponent
		| TableComponent
		| ButtonComponent
		| PieChartComponent
		| ImageComponent
		| InputComponent
		| SelectComponent
		| CheckboxComponent
		| RadioComponent
		| FormComponent
	)

export type ComponentSet = {
	title: string
	components: AppComponent[]
}

type SectionID = string

export type AppSection = {
	components: AppComponent[]
	id: SectionID
}

export type GridItem = FilledItem<{
	data: AppComponent
	id: string
}>

export type InlineScript = {
	content: string
	language: Preview.language
	path: string
	schema: Schema
}

export type App = {
	grid: GridItem[]
	title: string
	unusedInlineScripts: Record<string, InlineScript>
}

export type ConnectingInput = {
	opened: boolean
	input?: ConnectedInput
	sourceName?: string
}

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	app: Writable<App>
	selectedComponent: Writable<string | undefined>
	mode: Writable<EditorMode>
	connectingInput: Writable<ConnectingInput>
	breakpoint: Writable<EditorBreakpoint>
	runnableComponents: Writable<Record<string, () => void>>
	appPath: string
}

export type EditorMode = 'dnd' | 'preview'
export type EditorBreakpoint = 'sm' | 'lg'

type ComponentID = string
