import type { Schema } from '$lib/common'
import type { Preview } from '$lib/gen'
import type { FilledItem } from 'svelte-grid'
import type { Writable } from 'svelte/store'
import type {
	AppInput,
	ConnectedAppInput,
	ConnectedInput,
	EvalAppInput,
	RowAppInput,
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
export type SliderComponent = BaseComponent<'slidercomponent'>
export type HtmlComponent = BaseComponent<'htmlcomponent'>
export type VegaLiteComponent = BaseComponent<'vegalitecomponent'>
export type TimeseriesComponent = BaseComponent<'timeseriescomponent'>
export type ButtonComponent = BaseComponent<'buttoncomponent'> & {
	recomputeIds: string[] | undefined
}

export type FormComponent = BaseComponent<'formcomponent'> & {
	recomputeIds: string[] | undefined
}

export type FormButtonComponent = BaseComponent<'formbuttoncomponent'> & {
	recomputeIds: string[] | undefined
}

export type RunFormComponent = BaseComponent<'runformcomponent'>
export type BarChartComponent = BaseComponent<'barchartcomponent'>
export type PieChartComponent = BaseComponent<'piechartcomponent'>
export type ScatterChartComponent = BaseComponent<'scatterchartcomponent'>
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
	configuration: Record<
		string,
		(StaticAppInput | ConnectedAppInput | UserAppInput | RowAppInput | EvalAppInput) & {
			onlyStatic?: boolean
			evaluatedValue?: boolean
			tooltip?: string
		}
	>
	card: boolean | undefined
	/**
	 * If `true` then the wrapper will allow items to flow outside of it's borders.
	 *
	 * *For example when the component has a popup like `Select`*
	 */
	softWrap?: boolean
}

export type AppComponent = BaseAppComponent &
	(
		| RunFormComponent
		| DisplayComponent
		| TextInputComponent
		| PasswordInputComponent
		| DateInputComponent
		| NumberInputComponent
		| SliderComponent
		| BarChartComponent
		| TimeseriesComponent
		| HtmlComponent
		| TableComponent
		| TextComponent
		| TableComponent
		| ButtonComponent
		| PieChartComponent
		| ScatterChartComponent
		| ImageComponent
		| InputComponent
		| SelectComponent
		| CheckboxComponent
		| RadioComponent
		| FormComponent
		| FormButtonComponent
		| VegaLiteComponent
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
	fullscreen: boolean
	unusedInlineScripts: Array<{
		name: string
		inlineScript: InlineScript
	}>
	hiddenInlineScripts: Array<{
		name: string
		inlineScript: InlineScript | undefined
		fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	}>
}

export type ConnectingInput = {
	opened: boolean
	input?: ConnectedInput
	sourceName?: string
	hoveredComponent: string | undefined
}

export type AppEditorContext = {
	worldStore: Writable<World | undefined>
	staticOutputs: Writable<Record<string, string[]>>
	lazyGrid: Writable<GridItem[]>
	app: Writable<App>
	summary: Writable<string>
	selectedComponent: Writable<string | undefined>
	mode: Writable<EditorMode>
	connectingInput: Writable<ConnectingInput>
	breakpoint: Writable<EditorBreakpoint>
	runnableComponents: Writable<Record<string, () => Promise<void>>>
	staticExporter: Writable<Record<string, () => any>>
	appPath: string
	workspace: string
	onchange: (() => void) | undefined
	isEditor: boolean
	jobs: Writable<{ job: string; component: string }[]>
	noBackend: boolean
}

export type EditorMode = 'dnd' | 'preview'
export type EditorBreakpoint = 'sm' | 'lg'

type ComponentID = string
