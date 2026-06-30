import type { Policy } from '$lib/gen'
import type { History } from '$lib/history.svelte'

import type { Writable } from 'svelte/store'
import type {
	AppComponent,
	PresetComponentConfig,
	RecomputeOthersSource,
	components
} from './editor/component/components'
import type {
	AppInput,
	ConnectedAppInput,
	ConnectedInput,
	EvalAppInput,
	EvalV2AppInput,
	InlineScript,
	InputConnection,
	ResultAppInput,
	RowAppInput,
	Runnable,
	StaticAppInput,
	TemplateV2AppInput,
	UploadAppInput,
	UploadS3AppInput,
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
	tooltip?: string
	placeholder?: string
	customTitle?: string
}

export type ComponentCssProperty = {
	class?: string
	style?: string
	evalClass?: RichConfiguration
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
	| EvalV2AppInput
	| UploadAppInput
	| UploadS3AppInput
	| ResultAppInput
	| TemplateV2AppInput

export type StaticConfiguration = GeneralAppInput & StaticAppInput
export type OneOfRichConfiguration<T> = {
	type: 'oneOf'
	selected: string
	tooltip?: string
	labels?: Record<string, string>
	configuration: Record<string, Record<string, T>>
}

export type OneOfConfiguration = OneOfRichConfiguration<
	GeneralAppInput & (StaticAppInput | EvalAppInput | EvalV2AppInput)
>

export type RichConfigurationT<T> = (T & { type: AppInput['type'] }) | OneOfRichConfiguration<T>
export type RichConfiguration = RichConfigurationT<Configuration>
export type RichConfigurations = Record<string, RichConfiguration>

export type StaticRichConfigurations = Record<
	string,
	RichConfigurationT<GeneralAppInput & (StaticAppInput | EvalAppInput | EvalV2AppInput)>
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
	presets?: Readonly<PresetComponentConfig['type'][]> | undefined
}

type SectionID = string

export type AppSection = {
	components: AppComponent[]
	id: SectionID
}

export type GridItem = FilledItem<AppComponent>

export type AppCssItemName = 'viewer' | 'grid' | AppComponent['type']

export type HiddenRunnable = {
	name: string
	transformer?: InlineScript & { language: 'frontend' }
	// inlineScript?: InlineScript | undefined
	// type?: 'runnableByName' | 'runnableByPath'
	fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	autoRefresh?: boolean
	//deprecated and to be removed after migration
	doNotRecomputeOnInputChanged?: boolean
	recomputeOnInputChanged?: boolean
	noBackendValue?: any
	hidden?: boolean
} & Runnable &
	RecomputeOthersSource

export type AppTheme =
	| {
			type: 'path'
			path: string
	  }
	| {
			type: 'inlined'
			css: string
	  }

import type { DiffDrawerI } from '$lib/components/diff_drawer'

export interface AppEditorProps {
	app: App
	path: string
	policy: Policy
	summary: string
	/** Initial labels for the app, threaded from the loaded app data. */
	labels?: string[]
	/** Deployed app value the autosave `discardIf` compares against, so an
	 * edit reverting to deployed clears the draft instead of leaving a no-op.
	 * `undefined` for draft-only paths (no deployed baseline). */
	deployedBaseline?: App | undefined
	fromHub?: boolean
	diffDrawer?: DiffDrawerI | undefined
	savedApp?:
		| {
				value: App
				path: string
				summary: string
				policy: any
				custom_path?: string
				labels?: string[]
		  }
		| undefined
	version?: number | undefined
	newApp?: boolean
	newPath?: string | undefined
	replaceStateFn?: (path: string) => void
	gotoFn?: (path: string, opt?: Record<string, any> | undefined) => void
	onSavedNewAppPath?: (path: string) => void
	/** Override breadcrumb-picker navigation. Defaults to goto(editPathFor(item)). */
	onNavigate?: (item: import('$lib/components/workspacePicker').WorkspaceItem) => void
	// Threaded through `AppEditorHeader` to the `AutosaveIndicator`
	// popover so its "Reset to deployed" button can do the same thing
	// the load-time toast offers.
	onResetToDeployed?: () => void | Promise<void>
	// See ScriptBuilderProps — same semantics for the app editor's
	// indicator. Threaded through AppEditorHeader.
	loadedFromDraft?: boolean
	othersDraftsCount?: number
	onOpenOthersDrafts?: () => void
	// Restoring an older deployment from the history drawer. Threaded through
	// AppEditorHeader as a callback prop rather than `on:restore` forwarding,
	// which does not propagate through these runes-mode components.
	onRestore?: (restoredApp: any) => void
}

export type App = {
	grid: GridItem[]
	darkMode?: boolean
	fullscreen: boolean
	norefreshbar?: boolean
	unusedInlineScripts: Array<{
		name: string
		inlineScript: InlineScript
	}>

	//TODO: should be called hidden runnables but migration tbd
	hiddenInlineScripts: Array<HiddenRunnable>
	css?: Partial<Record<AppCssItemName, Record<string, ComponentCssProperty>>>
	subgrids?: Record<string, GridItem[]>
	theme: AppTheme | undefined
	lazyInitRequire?: string[] | undefined
	eagerRendering?: boolean | undefined
	hideLegacyTopBar?: boolean | undefined
	mobileViewOnSmallerScreens?: boolean | undefined
	version?: number
	/**
	 * Fork base for the stale-draft check: the deployed app version this draft
	 * was started from, pinned at fork. Stamped on the draft seed in the editor;
	 * compared against the deployed head (`versions[last]`) in the compare view.
	 * In DRAFT_COMPARE_IGNORED_FIELDS so it never trips the autosave no-op check.
	 */
	parent_version?: number
	/**
	 * User-typed path persisted on the autosaved App when it differs from
	 * the deployed/seeded baseline. The home list renders it so a friendly
	 * name shows up instead of the autogenerated `u/{user}/draft_{uuid}`
	 * URL slot for renames-in-progress and brand-new drafts. Dropped from
	 * the JSON once the typed path matches the baseline again, and deploy
	 * clears the whole draft.
	 */
	draft_path?: string
	/**
	 * App summary persisted on the autosaved App so a draft round-trips it — the
	 * autosave stores the bare App value, which otherwise drops the summary (it
	 * normally lives in the `app` table column, set only on deploy). Mirrors
	 * `draft_path`: draft-only metadata; the deployed summary column is
	 * authoritative, and the Review & Deploy page reads it from the draft and
	 * sends it as the summary on deploy.
	 */
	summary?: string
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

export type ListContext = Writable<{
	index: number
	value: any
	disabled: boolean
}>

export type ListInputs = {
	set: (id: string, value: any) => void
	remove: (id: string) => void
}

export type GroupContext = { id: string; context: Writable<Record<string, any>> }

export type JobById = {
	job: string
	component: string
	result?: any
	error?: any
	transformer?: { result?: any; error?: string }
	created_at?: number
	started_at?: number
	duration_ms?: number
}

export type AppViewerContext = {
	worldStore: Writable<World>
	app: Writable<App>
	summary: Writable<string>
	initialized: Writable<{
		initializedComponents: string[]
		initialized: boolean
		runnableInitialized: Record<string, any>
	}>
	selectedComponent: Writable<string[] | undefined>
	mode: Writable<EditorMode>
	connectingInput: Writable<ConnectingInput>
	breakpoint: Writable<EditorBreakpoint>
	bgRuns: Writable<string[]>
	runnableComponents: Writable<
		Record<
			string,
			{
				autoRefresh: boolean
				refreshOnStart?: boolean
				cb: ((inlineScript?: InlineScript, setRunnableJob?: boolean) => CancelablePromise<void>)[]
			}
		>
	>
	staticExporter: Writable<Record<string, () => any>>
	appPath: Writable<string>
	workspace: string
	onchange: (() => void) | undefined
	isEditor: boolean
	jobs: Writable<string[]>
	// jobByComponent: Writable<Record<string, string>>,
	jobsById: Writable<Record<string, JobById>>
	noBackend: boolean
	errorByComponent: Writable<Record<string, { id?: string; error: string }>>
	openDebugRun: Writable<((jobID: string) => void) | undefined>
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
				setValue?: (value: any) => void
				setSelectedIndex?: (index: number) => void
				openModal?: () => void
				closeModal?: () => void
				open?: () => void
				close?: () => void
				validate?: (key: string) => void
				invalidate?: (key: string, error: string) => void
				validateAll?: () => void
				clearFiles?: () => void
				sendMessage?: (message: string) => void
				showToast?: (message: string, error?: boolean) => void
				recompute?: () => void
				askNewResource?: () => void
				setGroupValue?: (key: string, value: any) => void
			}
		>
	>
	hoverStore: Writable<string | undefined>
	allIdsInPath: Writable<string[]>
	darkMode: Writable<boolean>
	cssEditorOpen: Writable<boolean>
	previewTheme: Writable<string | undefined>
	debuggingComponents: Writable<Record<string, number>>
	replaceStateFn?: ((url: string) => void) | undefined
	gotoFn?: ((url: string, opt?: Record<string, any> | undefined) => void) | undefined
	policy: Policy

	recomputeAllContext: Writable<{
		onRefresh?: (excludeId?: string) => void
		componentNumber?: number | undefined
		interval?: number | undefined
		refreshing?: string[] | undefined
		setInter?: (interval: number) => void | undefined
		progress?: number | undefined
		loading?: boolean | undefined
	}>
	panzoomActive: Writable<boolean>
}

export type AppEditorContext = {
	yTop: Writable<number>
	runnableJobEditorPanel: Writable<{
		focused: boolean
		jobs: Record<string, string>
		frontendJobs: Record<string, any>
		width: number
	}>
	evalPreview: Writable<Record<string, any>>
	componentActive: Writable<boolean>
	dndItem: Writable<Record<string, (x: number, y: number, topY: number) => void>>
	refreshComponents: Writable<(() => void) | undefined>
	history: History<App> | undefined
	pickVariableCallback: Writable<((path: string) => void) | undefined>
	selectedComponentInEditor: Writable<string | undefined>
	movingcomponents: Writable<string[] | undefined>
	jobsDrawerOpen: Writable<boolean>
	scale: Writable<number>
	stylePanel: () => any
}

export type FocusedGrid = { parentComponentId: string; subGridIndex: number }
export type EditorMode = 'dnd' | 'preview'
export type EditorBreakpoint = 'sm' | 'lg'

export const IS_APP_PUBLIC_CONTEXT_KEY = 'isAppPublicContext' as const

// Set by PublicAppFrame in opaque-viewer mode (WIN-2006). Lets the app relay
// top-level navigations (e.g. navbar links to another app) to the embedder,
// since navigating inside the opaque iframe would load the SPA cookieless.
export const EMBED_NAV_CONTEXT_KEY = 'appEmbedNav' as const
export type EmbedNav = { navigateTop: (href: string) => void }

type ComponentID = string

export type ContextPanelContext = {
	search: Writable<string>
	manuallyOpened: Writable<Record<string, boolean>>
	hasResult: Writable<Record<string, boolean>>
}
