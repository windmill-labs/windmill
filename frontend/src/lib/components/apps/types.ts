import type { Schema } from '$lib/common'
import type { Preview } from '$lib/gen'
import type { History } from '$lib/history'
import type { FilledItem } from '@windmill-labs/svelte-grid'
import type { Writable } from 'svelte/store'
import type { AppComponent } from './editor/component/components'
import type {
	AppInput,
	ConnectedAppInput,
	ConnectedInput,
	EvalAppInput,
	RowAppInput,
	StaticAppInput,
	UploadAppInput,
	UserAppInput
} from './inputType'
import type { World } from './rx'

export type HorizontalAlignment = 'left' | 'center' | 'right'
export type VerticalAlignment = 'top' | 'center' | 'bottom'

export type Aligned = {
	horizontalAlignment: HorizontalAlignment
	verticalAlignment: VerticalAlignment
}

export interface GeneralAppInput {
	onlyStatic?: boolean
	evaluatedValue?: boolean
	tooltip?: string
}

export type ComponentCssProperty = {
	class?: string
	style?: string
}

export type ComponentCustomCSS<T extends string = string> = Record<T, ComponentCssProperty>

export interface BaseAppComponent extends Partial<Aligned> {
	id: ComponentID
	componentInput: AppInput | undefined
	configuration: Record<
		string,
		GeneralAppInput &
		(
			| StaticAppInput
			| ConnectedAppInput
			| UserAppInput
			| RowAppInput
			| EvalAppInput
			| UploadAppInput
		)
	>
	card: boolean | undefined
	customCss?: ComponentCustomCSS
	/**
	 * If `true` then the wrapper will allow items to flow outside of it's borders.
	 *
	 * *For example when the component has a popup like `Select`*
	 */
	softWrap?: boolean
	// Number of subgrids
	numberOfSubgrids?: number
}

export type ComponentSet = {
	title: string
	components: Readonly<AppComponent['type'][]>
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
	css?: Record<'viewer' | 'grid' | AppComponent['type'], ComponentCustomCSS>
	subgrids?: Record<string, GridItem[]>
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
	errorByComponent: Writable<Record<string, { error: string; componentId: string }>>
	openDebugRun: Writable<((componentID: string) => void) | undefined>
	focusedGrid: Writable<FocusedGrid | undefined>
	stateId: Writable<number>
	parentWidth: Writable<number>
	history: History<App>
}

export type FocusedGrid = { parentComponentId: string; subGridIndex: number }
export type EditorMode = 'dnd' | 'preview'
export type EditorBreakpoint = 'sm' | 'lg'

export const IS_APP_PUBLIC_CONTEXT_KEY = 'isAppPublicContext' as const

type ComponentID = string
