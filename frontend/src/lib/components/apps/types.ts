import type { Schema } from '$lib/common'
import type { Preview } from '$lib/gen'
import type { History } from '$lib/history'

import type { Writable } from 'svelte/store'
import type { AppComponent, components } from './editor/component/components'
import type {
	AppInput,
	ConnectedAppInput,
	ConnectedInput,
	EvalAppInput,
	InputConnection,
	ResultAppInput,
	RowAppInput,
	StaticAppInput,
	UploadAppInput,
	UserAppInput
} from './inputType'
import type { World } from './rx'
import type { FilledItem } from './svelte-grid/types'

export type HorizontalAlignment = 'left' | 'center' | 'right'
export type VerticalAlignment = 'top' | 'center' | 'bottom'

export type Aligned = {
	horizontalAlignment: HorizontalAlignment
	verticalAlignment: VerticalAlignment
}

export interface GeneralAppInput {
	onlyStatic?: boolean
	tooltip?: string
	placeholder?: string
	customTitle?: string
}

export type ComponentCssProperty = {
	class?: string
	style?: string
}

export type ComponentCustomCSS<T extends keyof typeof components> = Partial<
	(typeof components)[T]['customCss']
>

export type Configuration =
	| StaticAppInput
	| ConnectedAppInput
	| UserAppInput
	| RowAppInput
	| EvalAppInput
	| UploadAppInput
	| ResultAppInput

export type StaticConfiguration = GeneralAppInput & StaticAppInput
export type RichConfigurationT<T> =
	| (T & { type: AppInput['type'] })
	| {
			type: 'oneOf'
			selected: string
			tooltip?: string
			labels?: Record<string, string>
			configuration: Record<string, Record<string, T>>
	  }
export type RichConfiguration = RichConfigurationT<Configuration>
export type RichConfigurations = Record<string, RichConfiguration>

export type StaticRichConfigurations = Record<
	string,
	RichConfigurationT<GeneralAppInput & (StaticAppInput | EvalAppInput)>
>

export interface BaseAppComponent extends Partial<Aligned> {
	id: ComponentID
	componentInput: AppInput | undefined
	configuration: RichConfigurations
	customCss?: Record<string, ComponentCssProperty>
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

export type GridItem = FilledItem<AppComponent>

export type InlineScript = {
	content: string
	language: Preview.language | 'frontend'
	path?: string
	schema?: Schema
	refreshOn?: { id: string; key: string }[]
}

export type AppCssItemName = 'viewer' | 'grid' | AppComponent['type']

export type HiddenInlineScript = {
	name: string
	inlineScript: InlineScript | undefined
	fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	autoRefresh?: boolean
	//deprecated and to be removed after migration
	doNotRecomputeOnInputChanged?: boolean
	recomputeOnInputChanged?: boolean
	noBackendValue?: any
	hidden?: boolean
}

export type App = {
	grid: GridItem[]
	fullscreen: boolean
	norefreshbar?: boolean
	unusedInlineScripts: Array<{
		name: string
		inlineScript: InlineScript
	}>
	hiddenInlineScripts: Array<HiddenInlineScript>
	css?: Partial<Record<AppCssItemName, Record<string, ComponentCssProperty>>>
	subgrids?: Record<string, GridItem[]>
}

export type ConnectingInput = {
	opened: boolean
	input?: ConnectedInput
	sourceName?: string
	hoveredComponent: string | undefined
	onConnect?: ((connection: InputConnection) => void) | undefined
}

export interface CancelablePromise<T> extends Promise<T> {
	cancel: () => void
}

export type AppViewerContext = {
	worldStore: Writable<World>
	app: Writable<App>
	summary: Writable<string>
	initialized: Writable<{
		initializedComponents: string[]
		initialized: boolean
	}>
	selectedComponent: Writable<string[] | undefined>
	mode: Writable<EditorMode>
	connectingInput: Writable<ConnectingInput>
	breakpoint: Writable<EditorBreakpoint>
	runnableComponents: Writable<
		Record<
			string,
			{
				autoRefresh: boolean
				refreshOnStart?: boolean
				cb: (inlineScript?: InlineScript) => CancelablePromise<void>
			}
		>
	>
	staticExporter: Writable<Record<string, () => any>>
	appPath: string
	workspace: string
	onchange: (() => void) | undefined
	isEditor: boolean
	jobs: Writable<{ job: string; component: string; result?: string; error?: string }[]>
	noBackend: boolean
	errorByComponent: Writable<Record<string, { error: string; componentId: string }>>
	openDebugRun: Writable<((componentID: string) => void) | undefined>
	focusedGrid: Writable<FocusedGrid | undefined>
	stateId: Writable<number>
	parentWidth: Writable<number>
	state: Writable<Record<string, any>>
	componentControl: Writable<
		Record<
			string,
			{
				left?: () => boolean
				right?: (skipTableActions?: boolean | undefined) => string | boolean
				setTab?: (index: number) => void
				agGrid?: { api: any; columnApi: any }
				setCode?: (value: string) => void
				onDelete?: () => void
			}
		>
	>
	hoverStore: Writable<string | undefined>
	allIdsInPath: Writable<string[]>
}

export type AppEditorContext = {
	history: History<App> | undefined
	pickVariableCallback: Writable<((path: string) => void) | undefined>
	ontextfocus: Writable<(() => void) | undefined>
	selectedComponentInEditor: Writable<string | undefined>
	movingcomponents: Writable<string[] | undefined>
}

export type FocusedGrid = { parentComponentId: string; subGridIndex: number }
export type EditorMode = 'dnd' | 'preview'
export type EditorBreakpoint = 'sm' | 'lg'

export const IS_APP_PUBLIC_CONTEXT_KEY = 'isAppPublicContext' as const

type ComponentID = string

export type ContextPanelContext = {
	search: Writable<string>
	manuallyOpened: Writable<Record<string, boolean>>
	hasResult: Writable<Record<string, boolean>>
}
