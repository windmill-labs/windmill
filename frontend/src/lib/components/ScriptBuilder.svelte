<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import {
		ScriptService,
		type Script,
		type NewScript,
		type TriggersCount,
		PostgresTriggerService,
		CaptureService,
		type ScriptLang,
		WorkerService
	} from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import {
		initialCode,
		canHavePreprocessor,
		getPreprocessorFullCode,
		getMainFunctionPattern
	} from '$lib/script_helpers'
	import { isWorkflowAsCode } from './graph/wacToFlow'
	import AIFormSettings from './copilot/AIFormSettings.svelte'
	import {
		defaultScripts,
		enterpriseLicense,
		usedTriggerKinds,
		userStore,
		userWorkspaces,
		workerTags,
		workspaceStore
	} from '$lib/stores'
	import {
		emptySchema,
		emptyString,
		generateRandomString,
		orderedJsonStringify,
		readFieldsRecursively,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
	import Path from './Path.svelte'
	import { invalidateWorkspacePaths } from './PathNameAutocomplete.svelte'
	import { notifyContractWarnings } from './assets/AssetGraph/schemaContracts'
	import ScriptEditor from './ScriptEditor.svelte'
	import { Alert, Button, Drawer, SecondsInput, Tab, TabContent, Tabs } from './common'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage, Schema } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ErrorHandlerToggleButton from '$lib/components/details/ErrorHandlerToggleButton.svelte'
	import {
		Bug,
		CheckCircle,
		Code,
		DiffIcon,
		EllipsisVertical,
		Network,
		Plus,
		Rocket,
		Settings,
		Shuffle,
		Tag,
		X
	} from 'lucide-svelte'
	import { base } from '$lib/base'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { parsePipelineAnnotations } from './assets/AssetGraph/parsePipelineAnnotations'
	import DropdownV2 from './DropdownV2.svelte'
	import { type Item } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import Awareness from './Awareness.svelte'
	import { fade } from 'svelte/transition'
	import Popover from './Popover.svelte'
	import Toggle from './Toggle.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import Section from './Section.svelte'
	import Label from './Label.svelte'
	import type Editor from './Editor.svelte'
	import WorkerTagPicker from './WorkerTagPicker.svelte'
	import MetadataGen from './copilot/MetadataGen.svelte'
	import { writable } from 'svelte/store'
	import { defaultScriptLanguages, processLangs } from '$lib/scripts'
	import DefaultScripts from './DefaultScripts.svelte'
	import { getContext, onMount, setContext, tick, untrack } from 'svelte'
	import EditorHeader from './EditorHeader.svelte'
	import AutosaveIndicator from './AutosaveIndicator.svelte'
	import LabelsInput from './LabelsInput.svelte'

	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'
	import TriggersEditor from './triggers/TriggersEditor.svelte'
	import type { ScheduleTrigger, TriggerContext } from './triggers'
	import CaptureTable from './triggers/CaptureTable.svelte'
	import type { SavedAndModifiedValue } from './common/confirmationModal/unsavedTypes'
	import DeployButton from './DeployButton.svelte'
	import { type Trigger, deployTriggers, handleSelectTriggerFromKind } from './triggers/utils'
	import DraftTriggersConfirmationModal from './common/confirmationModal/DraftTriggersConfirmationModal.svelte'
	import { Triggers } from './triggers/triggers.svelte'
	import type { ScriptBuilderProps } from './script_builder'
	import WorkerTagSelect from './WorkerTagSelect.svelte'
	import type { ButtonType } from './common/button/model'
	import DebounceLimit from './flows/DebounceLimit.svelte'
	import { buildForkEditUrl, editInForkAllowed, editInForkLabel } from '$lib/utils/editInFork'
	import OnBehalfOfSelector, { type OnBehalfOfChoice } from './OnBehalfOfSelector.svelte'
	import WacExportDrawer from './scripts/WacExportDrawer.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { UserDraftDbSyncer } from '$lib/userDraftDbSyncer.svelte'

	let {
		script = $bindable(),
		fullyLoaded = true,
		initialPath = $bindable(''),
		userDraftPath = '',
		autosaveWorkspace = undefined,
		autosavePath = undefined,
		template = $bindable('script'),
		initialArgs = {},
		lockedLanguage = false,
		showMeta = false,
		neverShowMeta = false,
		diffDrawer = undefined,
		savedScript = $bindable(undefined),
		searchParams = new URLSearchParams(),
		disableHistoryChange = false,
		customUi = {},
		savedPrimarySchedule = undefined,
		functionExports = undefined,
		children,
		onDeploy,
		onDeployError,
		onSeeDetails,
		onNavigate,
		onTestJob,
		disableAi,
		testPanelCollapsed = false,
		initialPathChosen = false,
		onResetToDeployed,
		loadedFromDraft = false,
		othersDraftsCount = 0,
		onOpenOthersDrafts,
		condensedHeader = false
	}: ScriptBuilderProps = $props()

	// Top-bar button size + bar height. Condensed (session preview) uses the
	// smallest well-supported unified size (`sm`) so the bar is thinner.
	const headerBtnSize = $derived(condensedHeader ? 'sm' : 'md')

	export function getInitialAndModifiedValues(): SavedAndModifiedValue {
		return {
			savedValue: savedScript,
			modifiedValue: {
				...script,
				draft_triggers: structuredClone(triggersState.getDraftTriggersSnapshot())
			}
		}
	}

	// used for new scripts for captures
	const fakeInitialPath =
		'u/' +
		($userStore?.username?.includes('@')
			? $userStore?.username.split('@')[0].replace(/[^a-zA-Z0-9_]/g, '')
			: $userStore?.username) +
		'/' +
		generateRandomString(12)

	let deployedValue: Value | undefined = $state(undefined) // Value to diff against
	let deployedBy: string | undefined = $state(undefined) // Author
	let confirmCallback: () => void = $state(() => {}) // What happens when user clicks `override` in warning

	// Top-bar responsive collapse — container width, not viewport. Collapse the
	// right group (hide the ~200px tag select, icon-only Diff/Settings) before the
	// full group crowds the path into heavy truncation; ~900 is where the path
	// keeps a usable width given the right group's natural ~440px.
	let topbarWidth = $state(0)
	const compactTopbar = $derived(topbarWidth > 0 && topbarWidth < 900)

	// The workspace this editor operates on: deploy, save-draft, trigger loading
	// and the AutosaveIndicator all target it. Falls back to the full-page
	// editor's global store; the sessions preview overrides it to the session's
	// (forked) workspace, so an embedded editor acts on the session's fork rather
	// than the navigation workspace ($workspaceStore, which stays put). indicatorPath
	// is the matching draft path (URL path full-page, session target in preview).
	const opWorkspace = $derived(autosaveWorkspace ?? $workspaceStore)
	const indicatorPath = $derived(autosavePath ?? userDraftPath)

	// The shared `workerTags` store caches tags for the navigation workspace. A
	// session editor deploys to `opWorkspace` (a fork), so it keeps a local list to
	// gate/populate the tag picker without reading or clobbering the shared cache.
	const usesLocalTags = $derived(opWorkspace != undefined && opWorkspace !== $workspaceStore)
	let localWorkerTags = $state<string[] | undefined>(undefined)
	const scriptWorkerTags = $derived(usesLocalTags ? localWorkerTags : $workerTags)

	function getCompactMenuItems(): Item[] {
		const hasTags = (scriptWorkerTags?.length ?? 0) > 0
		return [
			...(customUi?.topBar?.tagEdit != false && hasTags
				? [
						{
							displayName: 'Worker tag',
							icon: Tag,
							action: () => {
								selectedTab = 'runtime'
								metadataOpen = true
							}
						}
					]
				: [])
		]
	}
	let open: boolean = $state(false) // Is confirmation modal open
	let args: Record<string, any> = $state(untrack(() => initialArgs)) // Test args input
	let selectedInputTab: 'main' | 'preprocessor' | 'diagram' = $state('main')
	let hasPreprocessor = $state(false)
	let preserveOnBehalfOf = $state(false)

	const WM_DEPLOYERS_GROUP = 'wm_deployers'
	let isDeployer = $derived($userStore?.groups?.includes(WM_DEPLOYERS_GROUP) ?? false)
	let canPreserve = $derived(!!$userStore?.is_admin || !!$userStore?.is_super_admin || isDeployer)
	let originalOnBehalfOfEmail = $derived(savedScript?.on_behalf_of_email)
	let onBehalfOfChoice: OnBehalfOfChoice = $state(undefined)
	let customOnBehalfOfEmail: string = $state('')

	let metadataOpen = $state(
		!untrack(() => neverShowMeta) &&
			(untrack(() => showMeta) ||
				untrack(() => searchParams).get('metadata_open') == 'true' ||
				(initialPath == '' &&
					untrack(() => searchParams).get('state') == undefined &&
					untrack(() => searchParams).get('collab') == undefined))
	)

	let editor: Editor | undefined = $state(undefined)
	let scriptEditor: ScriptEditor | undefined = $state(undefined)
	let captureTable: CaptureTable | undefined = $state(undefined)
	let wacExportDrawer: WacExportDrawer | undefined = $state(undefined)

	// Draft triggers confirmation modal
	let draftTriggersModalOpen = $state(false)
	let confirmDeploymentCallback: (triggersToDeploy: Trigger[]) => void = () => {}

	async function handleDraftTriggersConfirmed(event: CustomEvent<{ selectedTriggers: Trigger[] }>) {
		const { selectedTriggers } = event.detail
		// Continue with saving the flow
		draftTriggersModalOpen = false
		confirmDeploymentCallback(selectedTriggers)
	}

	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(
		untrack(() => savedPrimarySchedule)
	) // keep for legacy
	const triggersCount = writable<TriggersCount | undefined>(
		untrack(() => savedPrimarySchedule)
			? {
					schedule_count: 1,
					primary_schedule: { schedule: untrack(() => savedPrimarySchedule)!.cron }
				}
			: undefined
	)
	const simplifiedPoll = writable(false)

	export function setPrimarySchedule(schedule: ScheduleTrigger | undefined | false) {
		primaryScheduleStore.set(schedule)
		loadTriggers()
	}

	export function setDraftTriggers(triggers: Trigger[] | undefined) {
		triggersState.setTriggers([
			...(triggers ?? []),
			...triggersState.triggers.filter((t) => !t.draftConfig)
		])
		loadTriggers()
	}

	onMount(() => {
		if (functionExports) {
			console.log('functionExports set')
			functionExports({
				setPreviewArgs: (args: Record<string, any>) => {
					scriptEditor?.setArgs(args)
				},
				runPreview: async () => await scriptEditor?.runTest(),
				setCode: (code: string, language?: Script['language']) => {
					if (language) {
						script.language = language
					}
					editor?.setCode(code)
				},
				getCode: () => editor?.getCode() ?? ''
			})
		}
	})

	async function loadTriggers() {
		if (!initialPath) {
			return
		}
		$triggersCount = await ScriptService.getTriggersCountOfScript({
			workspace: opWorkspace!,
			path: initialPath
		})

		await triggersState.fetchTriggers(
			triggersCount,
			opWorkspace,
			initialPath,
			false,
			$primaryScheduleStore,
			$userStore
		)
	}

	// Add triggers context store
	const triggersState = $state(
		new Triggers([
			{ type: 'webhook', path: '', isDraft: false },
			{ type: 'default_email', path: '', isDraft: false },
			...(script.draft_triggers ?? [])
		])
	)

	const captureOn = writable<boolean | undefined>(undefined)
	const showCaptureHint = writable<boolean | undefined>(undefined)
	setContext<TriggerContext>('TriggerContext', {
		triggersCount,
		simplifiedPoll,
		showCaptureHint: showCaptureHint,
		triggersState
	})

	const enterpriseLangs = ['bigquery', 'snowflake', 'mssql', 'oracledb']

	// Languages the pipeline editor treats as warehouse/dataset transforms —
	// the ones where a `-- pipeline` annotation is a natural next step.
	const pipelineHintLangs = ['duckdb', 'postgresql', 'bigquery', 'snowflake', 'mysql', 'mssql']
	const pipelineHintDismissed = useLocalStorageValue(
		'pipelineScriptHintDismissed',
		false,
		'boolean'
	)
	let showPipelineHint = $derived(
		!pipelineHintDismissed.val &&
			(script.kind === 'script' || script.kind === undefined) &&
			pipelineHintLangs.includes(script.language ?? '') &&
			!parsePipelineAnnotations(script.content ?? '').inPipeline
	)

	export function setCode(code: string): void {
		editor?.setCode(code)
	}

	const scriptKindOptions: {
		value: Script['kind']
		title: string
		Icon: any
		desc?: string
		documentationLink?: string
	}[] = [
		{
			value: 'script',
			title: 'Action',
			Icon: Code
		},
		{
			value: 'trigger',
			title: 'Trigger',
			desc: 'First module of flows to trigger them based on external changes. These kind of scripts are usually running on a schedule to periodically look for changes.',
			documentationLink: 'https://www.windmill.dev/docs/flows/flow_trigger',
			Icon: Rocket
		},
		{
			value: 'approval',
			title: 'Approval',
			desc: 'Send notifications externally to ask for approval to continue a flow.',
			documentationLink: 'https://www.windmill.dev/docs/flows/flow_approval',
			Icon: CheckCircle
		},
		{
			value: 'failure',
			title: 'Error Handler',
			desc: 'Handle errors in flows after all retry attempts have been exhausted.',
			documentationLink: 'https://www.windmill.dev/docs/flows/flow_error_handler',
			Icon: Bug
		},
		{
			value: 'preprocessor',
			title: 'Preprocessor',
			desc: 'Transform incoming requests before they are passed to the main entrypoint.',
			documentationLink: 'https://www.windmill.dev/docs/core_concepts/preprocessors',
			Icon: Shuffle
		}
	]

	let pathError = $state('')
	let loadingSave = $state(false)

	// Lifts the route's `?new_draft=true` `stopSync` suspension, but only after the
	// stores-gated bind:path cascade (and, for an empty seed, `initContent` via
	// `markContentReady`) settles — resuming earlier posts the seed/auto-generated
	// path as the user's first "edit". `restarted` keeps re-entry idempotent.
	function scheduleRestartSync(
		path: string,
		opts?: { waitForContent?: boolean }
	): { markContentReady: () => void } {
		let contentReady = !opts?.waitForContent
		let storesReady = !!($userStore && $workspaceStore)
		let restarted = false
		async function tryRestart() {
			if (restarted || !contentReady || !storesReady) return
			// 500ms covers the bind:path cascade even on cold reload; two ticks
			// weren't enough (bind:path fired ~100ms after restart, posting an edit).
			await new Promise((r) => setTimeout(r, 500))
			if (restarted) return
			restarted = true
			UserDraft.restartSync('script', path)
		}
		if (!storesReady) {
			$effect(() => {
				if ($userStore && $workspaceStore) {
					storesReady = true
					untrack(() => void tryRestart())
				}
			})
		}
		void tryRestart()
		return {
			markContentReady() {
				contentReady = true
				void tryRestart()
			}
		}
	}

	if (script.content == '') {
		// Suspend autosave around the bootstrap mutations: seeding the template
		// content is a programmatic write, not the user's first edit. The handle
		// keys on `userDraftPath` (URL path), not the editor-displayed `initialPath`.
		UserDraft.stopSync('script', userDraftPath)
		if (template === 'wac_python') {
			script.modules = {
				'helper.py': {
					content: 'def main(a: str) -> str:\n    return f"hello {a}"\n',
					language: 'python3'
				}
			}
		} else if (template === 'wac_typescript') {
			script.modules = {
				'helper.ts': {
					content: 'export function main(a: string): string {\n  return `hello ${a}`\n}\n',
					language: 'bun'
				}
			}
		}
		const restarter = scheduleRestartSync(userDraftPath, { waitForContent: true })
		initContent(script.language, script.kind, template).finally(() => restarter.markContentReady())
	} else if (userDraftPath && untrack(() => searchParams).get('new_draft') == 'true') {
		// Pre-filled new-draft seed (fork "Copy of X", hub fork, URL/YAML import): no
		// template to seed, but the route still suspended autosave — lift it or the
		// draft never persists (autosave stays dead for the session).
		scheduleRestartSync(userDraftPath)
	}

	async function isTemplateScript() {
		const params = new URLSearchParams(window.location.search)
		const templateId = params.get('id')
		if (templateId === null) {
			return undefined
		}
		try {
			const templateScript = await PostgresTriggerService.getTemplateScript({
				workspace: opWorkspace!,
				id: templateId
			})
			return templateScript
		} catch (error) {
			sendUserToast(
				'An error occurred when trying to load your template script, please try again later',
				true
			)
		}
	}

	async function initContent(
		language: SupportedLanguage,
		kind: Script['kind'] | undefined,
		template:
			| 'pgsql'
			| 'mysql'
			| 'script'
			| 'docker'
			| 'powershell'
			| 'bunnative'
			| 'claudesandbox'
			| 'wac_python'
			| 'wac_typescript'
			| 'ci_test_bun'
			| 'ci_test_python'
	) {
		scriptEditor?.disableCollaboration()
		// Seed synchronously so a Deploy before the async template fetch resolves
		// doesn't run `inferArgs` on empty content and toast "Could not parse code".
		script.content = initialCode(language, kind, template, false)
		const templateScript = await isTemplateScript()
		if (templateScript) {
			script.content = initialCode(language, kind, template, true)
		}
		if (templateScript) {
			script.content += '\r\n' + templateScript
		}
		scriptEditor?.inferSchema(script.content, { nlang: language, resetArgs: true })
		if (script.content != editor?.getCode()) {
			setCode(script.content)
		}
	}

	async function handleEditScript(stay: boolean, deployMsg?: string): Promise<void> {
		scriptEditor?.flushModuleState()
		// Fetch latest version and fetch entire script after if needed
		let actual_parent_hash: string | undefined = undefined

		try {
			if (initialPath && initialPath != '') {
				actual_parent_hash = (
					await ScriptService.getScriptLatestVersion({
						workspace: opWorkspace!,
						path: initialPath
					})
				)?.script_hash
			}
		} catch (error) {
			//
		}

		// Usually when we create new script, we put current hash as a parent_hash
		// But if we specify parent_hash that is already used, than we get error
		// In order to fix it we make sure that client's understanding of parent_hash
		// is aligns with understanding of backend.
		if (actual_parent_hash == undefined || script.parent_hash == actual_parent_hash) {
			// Handle directly
			await editScript(stay, script.parent_hash!, deployMsg)
		} else {
			// Fetch entire script, since we need it to show Diff
			await syncWithDeployed()

			if (
				deployedValue &&
				script &&
				orderedJsonStringify({ ...deployedValue, hash: undefined }) ===
					orderedJsonStringify(
						replaceFalseWithUndefined({ ...script, hash: undefined, parent_hash: undefined })
					)
			) {
				await editScript(stay, actual_parent_hash, deployMsg)
			} else {
				// Handle through confirmation modal
				confirmCallback = async () => {
					open = false
					if (actual_parent_hash) {
						await editScript(stay, actual_parent_hash, deployMsg)
					} else {
						sendUserToast('Could not fetch latest version of the script', true)
					}
				}
				// Open confirmation modal
				open = true
			}
		}
	}

	async function syncWithDeployed() {
		const latestScript = await ScriptService.getScriptByPath({
			workspace: opWorkspace!,
			path: initialPath,
			withStarredInfo: true
		})

		deployedValue = replaceFalseWithUndefined({
			...latestScript,
			workspace_id: undefined,
			created_at: undefined,
			created_by: undefined,
			extra_perms: undefined,
			lock: undefined,
			lock_error_logs: undefined,
			parent_hashes: undefined
		})

		deployedBy = latestScript.created_by
	}

	async function editScript(
		stay: boolean,
		parentHash: string,
		deploymentMsg?: string,
		triggersToDeploy?: Trigger[]
	): Promise<void> {
		if (!triggersToDeploy) {
			// Check if there are draft triggers that need confirmation
			const draftTriggers = triggersState.triggers.filter((trigger) => trigger.draftConfig)
			if (draftTriggers.length > 0) {
				draftTriggersModalOpen = true
				confirmDeploymentCallback = async (triggersToDeploy: Trigger[]) => {
					await editScript(stay, parentHash, deploymentMsg, triggersToDeploy)
				}
				return
			}
		}

		loadingSave = true
		try {
			// Legacy drafts can carry `schema: {}` (no `properties`), which trips
			// `inferArgs` on `JSON.stringify(schema.properties)` and toasts "Could
			// not parse code". Backfill an empty schema so it parses.
			if (!script.schema || !(script.schema as any).properties) {
				script.schema = emptySchema()
			}
			try {
				const result = await inferArgs(
					script.language,
					script.content,
					script.schema as any,
					script.kind === 'preprocessor' ? 'preprocessor' : undefined
				)
				if (script.kind === 'preprocessor') {
					script.auto_kind = undefined
					script.has_preprocessor = undefined
				} else {
					script.auto_kind = result?.auto_kind || undefined
					script.has_preprocessor = result?.has_preprocessor || undefined
				}
			} catch (error) {
				sendUserToast(`Could not parse code, are you sure it is valid?`, true)
			}

			const newHash = await ScriptService.createScript({
				workspace: opWorkspace!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: parentHash,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind,
					tag: script.tag,
					envs: script.envs,
					dedicated_worker: script.dedicated_worker,
					concurrent_limit: script.concurrent_limit,
					concurrency_time_window_s: script.concurrency_time_window_s,
					debounce_key: emptyString(script.debounce_key) ? undefined : script.debounce_key,
					debounce_delay_s: script.debounce_delay_s,
					debounce_args_to_accumulate:
						script.debounce_args_to_accumulate && script.debounce_args_to_accumulate.length > 0
							? script.debounce_args_to_accumulate
							: undefined,
					max_total_debouncing_time: script.max_total_debouncing_time,
					max_total_debounces_amount: script.max_total_debounces_amount,
					cache_ttl: script.cache_ttl,
					cache_ignore_s3_path: script.cache_ignore_s3_path,
					ws_error_handler_muted: script.ws_error_handler_muted,
					priority: script.priority,
					restart_unless_cancelled: script.restart_unless_cancelled,
					timeout: script.timeout,
					concurrency_key: emptyString(script.concurrency_key) ? undefined : script.concurrency_key,
					visible_to_runner_only: script.visible_to_runner_only,
					auto_kind: script.auto_kind,
					has_preprocessor: script.has_preprocessor,
					deployment_message: deploymentMsg || undefined,
					on_behalf_of_email: script.on_behalf_of_email,
					preserve_on_behalf_of: preserveOnBehalfOf || undefined,
					assets: script.assets,
					modules: script.modules,
					labels: script.labels
				}
			})

			// New/updated path now exists server-side — drop the autocomplete
			// cache so it shows up immediately instead of after the 60s TTL.
			invalidateWorkspacePaths(opWorkspace!)

			// Authoritative save-time schema-contract check (pipelines gap #2b):
			// warn-only, post-commit so a self-produced target resolves to the
			// content just deployed. Fire-and-forget — must never gate the deploy.
			notifyContractWarnings(opWorkspace!, script.language, script.content)

			if (!initialPath) {
				await CaptureService.moveCapturesAndConfigs({
					workspace: opWorkspace!,
					path: fakeInitialPath,
					requestBody: {
						new_path: script.path
					},
					runnableKind: 'script'
				})
			}

			if (triggersToDeploy) {
				await deployTriggers(
					triggersToDeploy,
					opWorkspace,
					!!$userStore?.is_admin || !!$userStore?.is_super_admin,
					usedTriggerKinds,
					script.path,
					true
				)
			}

			const { draft_triggers: _, ...newScript } = structuredClone($state.snapshot(script))
			savedScript = structuredClone($state.snapshot(newScript))
			setDraftTriggers([])

			if (!disableHistoryChange) {
				history.replaceState(history.state, '', `/scripts/edit/${script.path}`)
			}
			// "Stay" deploys (explicit "Deploy & Stay here" or lib scripts) keep the
			// editor in place rather than navigating to the deployed item.
			const stayHere =
				stay ||
				(script.auto_kind === 'lib' &&
					script.kind !== 'preprocessor' &&
					!isWorkflowAsCode(script.content, script.language))
			if (stayHere) {
				// Re-pin parent_hash so the next deploy's conflict check is against
				// the version we just wrote.
				script.parent_hash = newHash
			}
			// Always notify on a successful deploy; the consumer decides whether to
			// navigate (route) or stay + sync the preview (session). Previously the
			// stay/lib branch skipped onDeploy, so session previews didn't sync after
			// a "Deploy & Stay here" or lib-script deploy.
			onDeploy?.({ path: script.path, hash: newHash, stay: stayHere })
		} catch (error) {
			onDeployError?.({ path: script.path, error })
			sendUserToast(`Error while saving the script: ${error.body || error.message}`, true)
		}
		loadingSave = false
	}

	// Ctrl/Cmd+S forces an immediate flush of the pending autosave. Flush Monaco
	// + `tick()` first so the last keystrokes reach the bindable before the
	// syncer flushes. No toast — the AutosaveIndicator narrates the result, and
	// `flush` never rejects (postSave routes errors to the failures map).
	async function saveDraft(): Promise<void> {
		if (!opWorkspace || !userDraftPath) return
		editor?.flushPendingChanges()
		await tick()
		await UserDraftDbSyncer.flush({
			workspace: opWorkspace,
			itemKind: 'script',
			path: userDraftPath
		})
	}

	// Materialize a brand-new script's draft before the session preview loads it by
	// path — an untouched new script never autosaved, so forcePersist is the only
	// thing that creates the row. Gated to never-deployed: forcePersist skips the
	// discardIf baseline, safe only when there is none.
	async function persistDraftForSession(): Promise<void> {
		await saveDraft()
		if (opWorkspace && userDraftPath && savedScript?.no_deployed === true) {
			await UserDraft.forcePersist('script', userDraftPath, { workspace: opWorkspace })
		}
	}

	// Inside an AI session pane (which injects an aiChatManager via context) the
	// extra deploy-dropdown options — Deploy & Stay here, Fork, Edit in workspace
	// fork, Exit & See details, Export — don't make sense: the session always
	// stays put and is already scoped to a fork. Diff is exposed as a standalone
	// top-bar button (rendered independently of the session pane), not here.
	const inSessionPane = !!getContext('aiChatManager')

	async function openDiffDrawer() {
		if (!savedScript) {
			return
		}
		await syncWithDeployed()

		const currentDraftTriggers = structuredClone(triggersState.getDraftTriggersSnapshot())

		const deployed = deployedValue ?? savedScript
		// `script` comes from the full DB row (since #9351), so it carries `lock`/`extra_perms`
		// while the deployed side is trimmed in `syncWithDeployed` — null them here to match
		// that strip. They stay in the shared `cleanValueProperties` so version-to-version and
		// folder workspace/fork diffs still surface lockfile / sharing-permission changes.
		const current = {
			...script,
			lock: undefined,
			extra_perms: undefined,
			draft_triggers: currentDraftTriggers
		}
		if (current.assets && !current.assets.length) delete current.assets

		diffDrawer?.openDrawer()
		diffDrawer?.setDiff({
			mode: 'normal',
			deployed,
			draft: savedScript['draft'],
			current
		})
	}

	function computeDropdownItems(
		initialPath: string,
		savedScript: ((Script | NewScript) & { no_deployed?: boolean }) | undefined
	) {
		let dropdownItems: { label: string; onClick: () => void }[] =
			initialPath != '' && customUi?.topBar?.extraDeployOptions != false
				? [
						...(!inSessionPane
							? [
									{
										label: 'Deploy & Stay here',
										onClick: () => {
											handleEditScript(true)
										}
									},
									{
										label: 'Fork',
										onClick: () => {
											window.open(`/scripts/add?template=${initialPath}`)
										}
									},
									...(!isCloudHosted() && editInForkAllowed(opWorkspace, $userWorkspaces)
										? [
												{
													label: editInForkLabel(opWorkspace, $userWorkspaces),
													onClick: () => {
														window.open(buildForkEditUrl('script', initialPath))
													}
												}
											]
										: [])
								]
							: []),
						...(!inSessionPane &&
						savedScript?.no_deployed !== true &&
						script.kind === 'script' &&
						!script.auto_kind
							? [
									{
										label: 'Exit & See details',
										onClick: () => {
											onSeeDetails?.({ path: initialPath })
										}
									}
								]
							: []),
						...(!inSessionPane && isWorkflowAsCode(script.content, script.language)
							? [
									{
										label: 'Export as YAML/JSON',
										onClick: () => {
											wacExportDrawer?.open(script)
										}
									}
								]
							: [])
					]
				: []

		if (
			!inSessionPane &&
			dropdownItems.length === 0 &&
			isWorkflowAsCode(script.content, script.language)
		) {
			dropdownItems = [
				{
					label: 'Export as YAML/JSON',
					onClick: () => {
						wacExportDrawer?.open(script)
					}
				}
			]
		}

		return dropdownItems.length > 0 ? dropdownItems : undefined
	}

	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
					event.preventDefault()
				}
				break
		}
	}

	let path: Path | undefined = $state(undefined)
	// Seed "path is already chosen" so the summary→path auto-slug (which only
	// runs for new scripts with initialPath == '') doesn't clobber a path the
	// caller pre-assigned. The session preview opens AI-created scripts as new
	// (empty initialPath) but with a path the AI already picked.
	let dirtyPath = $state(initialPathChosen)

	let selectedTab: 'metadata' | 'runtime' | 'ui' | 'triggers' = $state(
		(() => {
			if (customUi?.settingsPanel?.disableMetadata !== true) {
				// first option: either no custom UI or metadata is enabled
				return 'metadata'
			}
			if (customUi?.settingsPanel?.disableRuntime !== true) {
				return 'runtime'
			}
			if (customUi?.settingsPanel?.disableGeneratedUi !== true) {
				return 'ui'
			}
			if (customUi?.settingsPanel?.disableTriggers !== true) {
				return 'triggers'
			}
			return 'metadata'
		})()
	)

	setContext('disableTooltips', untrack(() => customUi)?.disableTooltips === true)

	function langToLanguage(lang: SupportedLanguage | 'docker' | 'bunnative'): SupportedLanguage {
		if (lang == 'docker') {
			return 'bash'
		}
		if (lang == 'bunnative') {
			return 'bun'
		}
		return lang
	}

	async function applyArgs(e) {
		selectedInputTab = e.detail.kind
		metadataOpen = false
		if (scriptEditor) {
			scriptEditor.updateArgs(e.detail.args ?? {})
		}
	}

	function openTriggers(ev) {
		metadataOpen = true
		selectedTab = 'triggers'
		handleSelectTriggerFromKind(triggersState, triggersCount, initialPath, ev.detail.kind)
		captureOn.set(true)
	}

	function addPreprocessor(e?: { detail?: { args: Record<string, any> } }) {
		const code = editor?.getCode()
		if (code) {
			const preprocessorCode = getPreprocessorFullCode(script.language, false)
			const mainPattern = getMainFunctionPattern(script.language)
			const mainIndex = code.indexOf(mainPattern)

			if (mainIndex === -1) {
				editor?.setCode(code + preprocessorCode)
			} else {
				editor?.setCode(
					code.slice(0, mainIndex) + preprocessorCode + '\n' + code.slice(mainIndex) + '\n\n'
				)
			}
		}
		selectedInputTab = 'preprocessor'

		// Apply provided args to the preprocessor
		if (e?.detail?.args && Object.keys(e.detail.args).length > 0) {
			args = { ...args, ...e.detail.args }
		}
	}

	function handleDeployTrigger(_trigger: Trigger) {}

	function onScriptLanguageTrigger(lang: 'docker' | 'bunnative' | ScriptLang) {
		if (lang == 'docker') {
			template = 'docker'
		} else if (lang == 'bunnative') {
			template = 'bunnative'
		} else {
			template = 'script'
		}
		let language = langToLanguage(lang)
		//
		initContent(language, script.kind, template)
		script.language = language
	}

	function onSummaryChange(value: string) {
		if (initialPath == '' && value?.length > 0 && !dirtyPath) {
			path?.setName(
				value
					.toLowerCase()
					.replace(/[^a-z0-9_]/g, '_')
					.replace(/-+/g, '_')
					.replace(/^-|-$/g, '')
			)
		}
	}
	$effect(() => {
		initialPath != '' && untrack(() => loadTriggers())
	})
	let langs = $derived(
		processLangs(script.language, $defaultScripts?.order ?? Object.keys(defaultScriptLanguages))
			.map((l) => [defaultScriptLanguages[l], l])
			.filter((x) => $defaultScripts?.hidden == undefined || !$defaultScripts.hidden.includes(x[1]))
			.filter((x) => {
				if (customUi?.settingsPanel?.metadata?.languages === undefined) {
					return true
				}
				return customUi.settingsPanel.metadata.languages.includes(x[1] as SupportedLanguage)
			}) as [string, SupportedLanguage | 'docker' | 'bunnative'][]
	)
	$effect(() => {
		;['collab', 'path'].forEach((x) => {
			if (searchParams.get(x)) {
				searchParams.delete(x)
			}
		})
	})

	// Mirror the draft triggers (held in a separate `triggersState` $state)
	// back into `script.draft_triggers` so the UserDraft autosave — which
	// deep-tracks `script` — picks them up. Pre-PR ScriptBuilder ran its own
	// localStorage autosave that explicitly snapshotted triggersState; the
	// switch to a unified UserDraft handle dropped that bridge.
	$effect(() => {
		readFieldsRecursively(triggersState.triggers)
		script.draft_triggers = triggersState.getDraftTriggersSnapshot()
	})

	loadWorkerTags()
	async function loadWorkerTags() {
		if (usesLocalTags) {
			if (!localWorkerTags) {
				localWorkerTags = await WorkerService.getCustomTagsForWorkspace({ workspace: opWorkspace! })
			}
		} else if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTagsForWorkspace({ workspace: opWorkspace! })
		}
	}
</script>

<svelte:window onkeydown={onKeyDown} />
{@render children?.()}

<DeployOverrideConfirmationModal
	{deployedBy}
	{confirmCallback}
	bind:open
	{diffDrawer}
	bind:deployedValue
	currentValue={script}
/>

<DraftTriggersConfirmationModal
	bind:open={draftTriggersModalOpen}
	draftTriggers={triggersState.triggers.filter((t) => t.draftConfig)}
	on:canceled={() => {
		draftTriggersModalOpen = false
	}}
	on:confirmed={handleDraftTriggersConfirmed}
/>

{#if !$userStore?.operator}
	<Drawer
		placement="right"
		bind:open={metadataOpen}
		size={selectedTab === 'ui' || selectedTab === 'triggers' ? '1200px' : '800px'}
	>
		<DrawerContent
			noPadding
			title="Settings"
			on:close={() => (metadataOpen = false)}
			aiId="script-builder-settings"
			aiDescription="Script builder settings"
		>
			<!-- svelte-ignore a11y_autofocus -->
			<div class="flex flex-col h-full">
				<Tabs bind:selected={selectedTab} wrapperClass="flex-none w-full">
					{#if customUi?.settingsPanel?.disableMetadata !== true}
						<Tab
							value="metadata"
							aiId="script-builder-metadata"
							aiDescription="Metadata settings"
							label="Metadata"
						/>
					{/if}
					{#if customUi?.settingsPanel?.disableRuntime !== true}
						<Tab
							value="runtime"
							aiId="script-builder-runtime"
							aiDescription="Runtime settings"
							label="Runtime"
						/>
					{/if}
					{#if customUi?.settingsPanel?.disableGeneratedUi !== true}
						<Tab
							value="ui"
							aiId="script-builder-ui"
							aiDescription="Generated UI settings"
							label="Generated UI"
						>
							{#snippet extra()}
								<Tooltip
									documentationLink="https://www.windmill.dev/docs/core_concepts/json_schema_and_parsing"
								>
									The arguments are synced with the main signature but you may refine the parts that
									cannot be inferred from the type directly.
								</Tooltip>
							{/snippet}
						</Tab>
					{/if}
					{#if customUi?.settingsPanel?.disableTriggers !== true}
						<Tab
							value="triggers"
							aiId="script-builder-triggers"
							aiDescription="Triggers settings"
							label="Triggers"
						>
							{#snippet extra()}
								<Tooltip documentationLink="https://www.windmill.dev/docs/getting_started/triggers">
									Configure how this script will be triggered.
								</Tooltip>
							{/snippet}
						</Tab>
					{/if}

					{#snippet content()}
						<div class="min-h-0 grow overflow-y-auto">
							<TabContent value="metadata">
								<div class="flex flex-col gap-8 px-4 py-2 pb-12">
									<Section label="Metadata">
										{#snippet action()}
											{#if customUi?.settingsPanel?.metadata?.disableMute !== true}
												<div class="flex flex-row items-center gap-2">
													<ErrorHandlerToggleButton
														kind="script"
														scriptOrFlowPath={script.path}
														bind:errorHandlerMuted={script.ws_error_handler_muted}
														iconOnly={false}
													/>
												</div>
											{/if}
										{/snippet}
										<div class="flex flex-col gap-6">
											<Label label="Summary">
												<MetadataGen
													aiId="create-script-summary-input"
													aiDescription="Summary / Title of the new script"
													bind:content={script.summary}
													code={script.content}
													promptConfigName="summary"
													generateOnAppear
													onChange={() => onSummaryChange(script.summary)}
													elementProps={{
														type: 'text',
														placeholder: 'Short summary to be displayed when listed'
													}}
												/>
											</Label>
											<LabelsInput bind:labels={script.labels} class="-mt-4" />
											<Label label="Path">
												{#snippet header()}
													<Tooltip
														documentationLink="https://www.windmill.dev/docs/core_concepts/roles_and_permissions#path"
													>
														The unique identifier of the script in the workspace that defines
														permissions
													</Tooltip>
												{/snippet}
												<Path
													bind:this={path}
													bind:error={pathError}
													bind:path={script.path}
													bind:dirty={dirtyPath}
													{initialPath}
													autofocus={false}
													namePlaceholder="script"
													kind="script"
													workspaceOverride={opWorkspace}
												/>
												{#if initialPath && script.path && script.path !== initialPath}
													<Alert
														type="info"
														size="xs"
														title="Deploy the script to make the path change effective."
														class="mt-2"
													/>
												{/if}
											</Label>
											<Label label="Description">
												<MetadataGen
													bind:content={script.description}
													code={script.content}
													promptConfigName="description"
													elementType="textarea"
													elementProps={{
														placeholder: 'What this script does and how to use it'
													}}
												/>
											</Label>
										</div>
									</Section>
									{#if !customUi?.settingsPanel?.metadata?.languages || customUi?.settingsPanel?.metadata?.languages?.length > 1}
										<Section label="Language">
											{#snippet action()}
												<DefaultScripts />
											{/snippet}
											{#if lockedLanguage}
												<div class="text-sm text-primary italic mb-2">
													As a forked script, the language '{script.language}' cannot be modified.
												</div>
											{/if}
											<div class=" grid grid-cols-3 gap-2">
												{#each langs as [label, lang] (lang)}
													{@const isPicked =
														(lang == script.language &&
															template != 'bunnative' &&
															template != 'docker' &&
															template != 'claudesandbox') ||
														(template == 'bunnative' && lang == 'bunnative') ||
														(template == 'docker' && lang == 'docker') ||
														(template == 'claudesandbox' && lang == 'bun')}
													<Popover
														disablePopup={!enterpriseLangs.includes(lang) || !!$enterpriseLicense}
													>
														<Button
															aiId={`create-script-language-button-${lang}`}
															aiDescription={`Choose ${lang} as the language of the script`}
															unifiedSize="lg"
															variant="default"
															selected={isPicked}
															btnClasses={isPicked ? '' : 'm-[1px]'}
															on:click={() => onScriptLanguageTrigger(lang)}
															disabled={lockedLanguage ||
																(enterpriseLangs.includes(lang) && !$enterpriseLicense) ||
																(script.kind == 'preprocessor' && !canHavePreprocessor(lang))}
															startIcon={{
																icon: LanguageIcon,
																props: { lang }
															} as ButtonType.Icon}
														>
															<span class="truncate">{label}</span>
														</Button>
														{#snippet text()}
															{label} is only available with an enterprise license
														{/snippet}
													</Popover>
												{/each}
											</div>
										</Section>
									{/if}
									<div class="flex items-center gap-2 mt-2 flex-wrap">
										<span class="text-2xs text-secondary">Template</span>
										<Button
											size="xs2"
											variant="border"
											color="light"
											startIcon={{
												icon: LanguageIcon,
												props: { lang: 'claudesandbox', width: 16, height: 16 }
											} as ButtonType.Icon}
											on:click={() => {
												template = 'claudesandbox'
												script.language = 'bun'
												initContent('bun', script.kind, template)
											}}
										>
											Claude Sandbox
										</Button>
										<span title="Workflow-as-Code">
											<Button
												size="xs2"
												variant="border"
												color="light"
												startIcon={{
													icon: LanguageIcon,
													props: { lang: 'bun', width: 16, height: 16 }
												} as ButtonType.Icon}
												on:click={() => {
													template = 'wac_typescript'
													script.language = 'bun'
													script.modules = {
														'helper.ts': {
															content:
																'export function main(a: string): string {\n  return `hello ${a}`\n}\n',
															language: 'bun'
														}
													}
													initContent('bun', script.kind, template)
												}}
											>
												WAC TypeScript
											</Button>
										</span>
										<span title="Workflow-as-Code">
											<Button
												size="xs2"
												variant="border"
												color="light"
												startIcon={{
													icon: LanguageIcon,
													props: { lang: 'python3', width: 16, height: 16 }
												} as ButtonType.Icon}
												on:click={() => {
													template = 'wac_python'
													script.language = 'python3'
													script.modules = {
														'helper.py': {
															content: 'def main(a: str) -> str:\n    return f"hello {a}"\n',
															language: 'python3'
														}
													}
													initContent('python3', script.kind, template)
												}}
											>
												WAC Python
											</Button>
										</span>
										<Button
											size="xs2"
											variant="border"
											color="light"
											startIcon={{
												icon: LanguageIcon,
												props: { lang: 'bun', width: 16, height: 16 }
											} as ButtonType.Icon}
											on:click={() => {
												template = 'ci_test_bun'
												script.language = 'bun'
												initContent('bun', script.kind, template)
											}}
										>
											CI Test TypeScript
										</Button>
										<Button
											size="xs2"
											variant="border"
											color="light"
											startIcon={{
												icon: LanguageIcon,
												props: { lang: 'python3', width: 16, height: 16 }
											} as ButtonType.Icon}
											on:click={() => {
												template = 'ci_test_python'
												script.language = 'python3'
												initContent('python3', script.kind, template)
											}}
										>
											CI Test Python
										</Button>
									</div>
									{#if customUi?.settingsPanel?.metadata?.disableScriptKind !== true}
										<Section label="Script kind">
											{#snippet header()}
												<Tooltip
													documentationLink="https://www.windmill.dev/docs/script_editor/script_kinds"
												>
													Tag this script's purpose within flows such that it is available as the
													corresponding action.
												</Tooltip>
											{/snippet}
											<ToggleButtonGroup
												selected={script.kind}
												on:selected={({ detail }) => {
													template = 'script'
													script.kind = detail
													initContent(script.language, detail, template)
												}}
											>
												{#snippet children({ item })}
													{#each scriptKindOptions as { value, title, desc, documentationLink, Icon }}
														<ToggleButton
															label={title}
															{value}
															tooltip={desc}
															{documentationLink}
															icon={Icon}
															showTooltipIcon={Boolean(desc)}
															{item}
														/>
													{/each}
												{/snippet}
											</ToggleButtonGroup>
										</Section>
									{/if}
									{#if customUi?.settingsPanel?.disableRuntime}
										<Section label="Worker group tag (queue)">
											<WorkerTagPicker
												bind:tag={script.tag}
												placeholder={customUi?.tagSelectPlaceholder}
											/>
										</Section>
									{/if}

									{#if script.schema && !disableAi && !customUi?.settingsPanel?.metadata?.disableAiFilling}
										<div class="mt-3">
											<AIFormSettings
												bind:prompt={script.schema.prompt_for_ai as string | undefined}
												type="script"
											/>
										</div>
									{/if}
								</div>
							</TabContent>
							<TabContent value="runtime">
								<div class="flex flex-col gap-8 px-4 py-2 pb-12">
									<Section label="Worker group tag (queue)">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups"
											>
												The script will be executed on a worker configured to listen to this worker
												group tag (queue). For instance, you could setup an "highmem", or "gpu" tag.
											</Tooltip>
										{/snippet}
										<WorkerTagPicker
											bind:tag={script.tag}
											placeholder={customUi?.tagSelectPlaceholder}
										/>
									</Section>

									<Section label="Concurrency limits" eeOnly>
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/concurrency_limits"
											>
												Allowed concurrency within a given timeframe
											</Tooltip>
										{/snippet}
										<Toggle
											size="sm"
											checked={Boolean(script.concurrent_limit)}
											on:change={() => {
												if (script.concurrent_limit && script.concurrent_limit != undefined) {
													script.concurrent_limit = undefined
													script.concurrency_time_window_s = undefined
													script.concurrency_key = undefined
												} else {
													script.concurrent_limit = 1
												}
											}}
											options={{
												right: 'Concurrency limits'
											}}
										/>
										{#if Boolean(script.concurrent_limit)}
											<div class="flex flex-col gap-4 mt-2">
												<Label label="Max number of executions within the time window">
													<div class="flex flex-row gap-2 max-w-sm whitespace-nowrap">
														<input
															disabled={!$enterpriseLicense}
															bind:value={script.concurrent_limit}
															type="number"
														/>
													</div>
												</Label>
												{#if Boolean(script.concurrent_limit)}
													<Label label="Time window in seconds">
														<SecondsInput
															disabled={!$enterpriseLicense}
															bind:seconds={script.concurrency_time_window_s}
														/>
													</Label>
													<Label label="Custom concurrency key (optional)">
														{#snippet header()}
															<Tooltip
																documentationLink="https://www.windmill.dev/docs/core_concepts/concurrency_limits#custom-concurrency-key"
															>
																Concurrency keys are global, you can have them be workspace specific
																using the variable `$workspace`. You can also use an argument's
																value using `$args[name_of_arg]`</Tooltip
															>
														{/snippet}
														<input
															disabled={!$enterpriseLicense}
															type="text"
															autofocus
															bind:value={script.concurrency_key}
															placeholder={`$workspace/script/${script.path}-$args[foo]`}
														/>
													</Label>
												{/if}
											</div>
										{/if}
									</Section>
									<Section label="Cache">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/caching"
											>
												Cache the results for each possible inputs
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<Toggle
												size="sm"
												bind:checked={
													() => !!script.cache_ttl, (v) => (script.cache_ttl = v ? 300 : undefined)
												}
												options={{ right: 'Cache the results for each possible inputs' }}
											/>
											{#if script.cache_ttl}
												<div class="text-2xs text-secondary">How long to keep the cache valid</div>
												<SecondsInput bind:seconds={script.cache_ttl} />
												<Toggle
													size="2xs"
													bind:checked={
														() => script.cache_ignore_s3_path,
														(v) => (script.cache_ignore_s3_path = v || undefined)
													}
													options={{
														right: 'Ignore S3 Object paths for caching purposes',
														rightTooltip:
															'If two S3 objects passed as input have the same content, they will hit the same cache entry, regardless of their path.'
													}}
												/>
											{/if}
										</div>
									</Section>
									<Section label="Timeout">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/script_editor/settings#timeout"
											>
												Add a custom timeout for this script
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<Toggle
												size="sm"
												checked={Boolean(script.timeout)}
												on:change={() => {
													if (script.timeout && script.timeout != undefined) {
														script.timeout = undefined
													} else {
														script.timeout = customUi?.defaultTimeout ?? 300
													}
												}}
												options={{
													right: 'Add a custom timeout for this script'
												}}
											/>
											{#if Boolean(script.timeout)}
												<span class="text-xs font-semibold text-emphasis leading-none mt-2">
													Timeout duration
												</span>
												{#if script.timeout}
													<SecondsInput bind:seconds={script.timeout} />
												{:else}
													<SecondsInput disabled />
												{/if}
											{/if}
										</div>
									</Section>
									<Section label="Debouncing">
										<DebounceLimit
											size="sm"
											bind:debounce_delay_s={script.debounce_delay_s}
											bind:debounce_key={script.debounce_key}
											bind:debounce_args_to_accumulate={script.debounce_args_to_accumulate}
											bind:max_total_debouncing_time={script.max_total_debouncing_time}
											bind:max_total_debounces_amount={script.max_total_debounces_amount}
											schema={script.schema as Schema}
											placeholder={`$workspace/script/${script.path}-$args[foo]`}
										/>

										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/job_debouncing"
											>
												Debounce Jobs
											</Tooltip>
										{/snippet}
									</Section>

									<Section label="Perpetual script">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/script_editor/perpetual_scripts"
											>
												Restart the script upon ending unless cancelled
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<Toggle
												size="sm"
												checked={Boolean(script.restart_unless_cancelled)}
												on:change={() => {
													if (script.restart_unless_cancelled) {
														script.restart_unless_cancelled = undefined
													} else {
														script.restart_unless_cancelled = true
													}
												}}
												options={{
													right: 'Restart upon ending unless cancelled'
												}}
											/>
										</div>
									</Section>
									<Section label="Dedicated workers" eeOnly>
										<Toggle
											disabled={!$enterpriseLicense ||
												isCloudHosted() ||
												(script.language != 'bun' &&
													script.language != 'bunnative' &&
													script.language != 'python3' &&
													script.language != 'deno')}
											size="sm"
											checked={Boolean(script.dedicated_worker)}
											on:change={() => {
												if (script.dedicated_worker) {
													script.dedicated_worker = undefined
												} else {
													script.dedicated_worker = true
												}
											}}
											options={{
												right: 'Script is run on dedicated workers'
											}}
										/>
										{#if script.dedicated_worker}
											<div class="py-2">
												<Alert type="info" title="Require dedicated workers">
													A worker group needs to be configured to listen to this script. Select it
													in the dedicated workers section of the worker group configuration.
												</Alert>
											</div>
										{/if}
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/dedicated_workers"
											>
												In this mode, the script is meant to be run on dedicated workers that run
												the script at native speed. Can reach &gt;1500rps per dedicated worker. Only
												available on enterprise edition and for Python3, Deno, Bun and Bunnative.
												For other languages, the efficiency is already on par with dedicated workers
												since they do not spawn a full runtime</Tooltip
											>
										{/snippet}
									</Section>
									<Section label="Delete after completion">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/script_editor/settings#delete-after-use"
											>
												WARNING: This settings ONLY applies to synchronous webhooks or when the
												script is used within a flow. If used individually, this script must be
												triggered using a synchronous endpoint to have the desired effect.
												<br />
												<br />
												The logs, arguments and results of the job will be completely deleted from Windmill
												after the specified delay once it is complete and the result has been returned.
												Set to 0 for immediate deletion.
												<br />
												<br />
												The deletion is irreversible.
												{#if !$enterpriseLicense}
													<br />
													<br />
													This option is only available on Windmill Enterprise Edition.
												{/if}
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<Toggle
												disabled={!$enterpriseLicense}
												size="sm"
												checked={script.delete_after_secs != null}
												on:change={() => {
													if (script.delete_after_secs != null) {
														script.delete_after_secs = undefined
													} else {
														script.delete_after_secs = 0
													}
												}}
												options={{
													right: 'Delete logs, arguments and results after completion'
												}}
											/>
											{#if script.delete_after_secs != null}
												<SecondsInput
													bind:seconds={script.delete_after_secs}
													disabled={!$enterpriseLicense}
												/>
											{/if}
										</div>
									</Section>
									{#if !isCloudHosted()}
										<Section label="High priority script" eeOnly>
											<Toggle
												disabled={!$enterpriseLicense || isCloudHosted()}
												size="sm"
												checked={script.priority !== undefined && script.priority > 0}
												on:change={() => {
													if (script.priority) {
														script.priority = undefined
													} else {
														script.priority = 100
													}
												}}
												options={{
													right: 'Label as high priority'
												}}
											>
												{#snippet right()}
													<input
														type="number"
														class="!w-16 ml-4"
														disabled={script.priority === undefined}
														bind:value={script.priority}
														onfocus={bubble('focus')}
														onchange={() => {
															if (script.priority && script.priority > 100) {
																script.priority = 100
															} else if (script.priority && script.priority < 0) {
																script.priority = 0
															}
														}}
													/>
												{/snippet}
											</Toggle>
											{#snippet header()}
												<!-- TODO: Add EE-only badge when we have it -->
												<Tooltip
													documentationLink="https://www.windmill.dev/docs/core_concepts/jobs#high-priority-jobs"
												>
													Jobs from script labeled as high priority take precedence over the other
													jobs when in the jobs queue.
													{#if !$enterpriseLicense}This is a feature only available on enterprise
														edition.{/if}
												</Tooltip>
											{/snippet}
										</Section>
									{/if}
									<Section label="Runs visibility">
										{#snippet header()}
											<Tooltip
												documentationLink="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs#invisible-runs"
											>
												When this option is enabled, manual executions of this script are invisible
												to users other than the user running it, including the owner(s). This
												setting can be overridden when this script is run manually from the advanced
												menu.
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<Toggle
												size="sm"
												checked={Boolean(script.visible_to_runner_only)}
												on:change={() => {
													if (script.visible_to_runner_only) {
														script.visible_to_runner_only = undefined
													} else {
														script.visible_to_runner_only = true
													}
												}}
												options={{
													right: 'Make runs invisible to others'
												}}
											/>
										</div>
									</Section>
									<Section
										label={canPreserve
											? 'Run on behalf of a specified user'
											: 'On behalf of last editor'}
									>
										{#snippet header()}
											<Tooltip>
												When this option is enabled, the script will be run with the permissions of
												{canPreserve ? 'the specified user' : 'the last editor'}.
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<span class="inline-flex gap-2">
												<Toggle
													size="sm"
													checked={Boolean(script.on_behalf_of_email)}
													on:change={() => {
														if (script.on_behalf_of_email) {
															script.on_behalf_of_email = undefined
															preserveOnBehalfOf = false
															onBehalfOfChoice = undefined
														} else {
															script.on_behalf_of_email = $userStore?.email
														}
													}}
													options={{
														right: `Run on behalf of ${canPreserve ? 'a specified user' : 'last editor'}`
													}}
												/>
												{#if script.on_behalf_of_email && canPreserve}
													&rarr; <OnBehalfOfSelector
														targetWorkspace={opWorkspace ?? ''}
														targetValue={originalOnBehalfOfEmail}
														selected={onBehalfOfChoice}
														onSelect={(choice, details) => {
															onBehalfOfChoice = choice
															if (choice === 'me') {
																script.on_behalf_of_email = $userStore?.email
																customOnBehalfOfEmail = ''
																preserveOnBehalfOf = false
															} else if (choice === 'target') {
																script.on_behalf_of_email = originalOnBehalfOfEmail
																customOnBehalfOfEmail = ''
																preserveOnBehalfOf = true
															} else if (choice === 'custom' && details) {
																script.on_behalf_of_email = details.email
																customOnBehalfOfEmail = details.email
																preserveOnBehalfOf = true
															}
														}}
														kind="script"
														{canPreserve}
														customValue={customOnBehalfOfEmail}
														isDeployment={false}
													/>
												{:else if script.on_behalf_of_email && !canPreserve}
													<span class="text-xs text-tertiary">
														Currently: <span class="font-medium"
															>{originalOnBehalfOfEmail ?? script.on_behalf_of_email}</span
														>. Will be set to <span class="font-medium">{$userStore?.email}</span> on
														deploy (requires admin or wm_deployers group to override)
													</span>
												{/if}
											</span>
										</div>
									</Section>
									{#if !isCloudHosted()}
										<Section label="Custom env variables">
											{#snippet header()}
												<Tooltip
													documentationLink="https://www.windmill.dev/docs/script_editor/custom_environment_variables"
												>
													Additional static custom env variables to pass to the script.
												</Tooltip>
											{/snippet}
											{#if script.envs && script.envs.length > 0}
												<Alert type="warning" title="Not passed in previews" size="xs">
													Static envs variables are not passed in preview but solely on deployed
													scripts.
												</Alert>
											{/if}
											<div class="w-full mt-2">
												<span class="text-primary text-xs pb-2">Format is: `{'<KEY>=<VALUE>'}`</span
												>
												{#if Array.isArray(script.envs ?? [])}
													{#each script.envs ?? [] as _v, i}
														<div class="flex max-w-md mt-1 w-full items-center relative">
															{#if script.envs}
																<input
																	type="text"
																	bind:value={script.envs[i]}
																	placeholder="<KEY>=<VALUE>"
																/>
															{/if}
															<button
																transition:fade|local={{ duration: 50 }}
																class="rounded-full p-1 bg-surface/60 duration-200 hover:bg-gray-200 absolute right-2"
																aria-label="Clear"
																onclick={() => {
																	script.envs && script.envs.splice(i, 1)
																	script.envs = script.envs
																}}
															>
																<X size={14} />
															</button>
														</div>
													{/each}
												{/if}
											</div>
											<div class="flex mt-2">
												<Button
													variant="default"
													size="xs"
													on:click={() => {
														if (script.envs == undefined || !Array.isArray(script.envs)) {
															script.envs = []
														}
														script.envs = script.envs.concat('')
													}}
												>
													<div class="flex flex-row gap-1">
														<Plus size="16" />
														Add item
													</div>
												</Button>
											</div>
										</Section>
									{/if}
								</div>
							</TabContent>
							<TabContent value="ui" class="h-full p-4">
								<ScriptSchema
									bind:schema={script.schema}
									customUi={customUi?.settingsPanel?.metadata?.editableSchemaForm}
								/>
							</TabContent>
							<TabContent value="triggers" class="h-full">
								<TriggersEditor
									on:applyArgs={applyArgs}
									on:addPreprocessor={addPreprocessor}
									on:exitTriggers={() => {
										captureTable?.loadCaptures(true)
									}}
									currentPath={script.path}
									{initialPath}
									{fakeInitialPath}
									noEditor={true}
									newItem={initialPath == ''}
									isFlow={false}
									{hasPreprocessor}
									canHavePreprocessor={canHavePreprocessor(script.language)}
									args={hasPreprocessor && selectedInputTab !== 'preprocessor' ? {} : args}
									isDeployed={savedScript && savedScript?.no_deployed !== true}
									schema={script.schema}
									runnableVersion={script.parent_hash}
									onDeployTrigger={handleDeployTrigger}
								/>

								<!-- <ScriptSchedules {initialPath} schema={script.schema} schedule={scheduleStore} /> -->
							</TabContent>
						</div>
					{/snippet}
				</Tabs>
			</div>
		</DrawerContent>
	</Drawer>

	<div class="flex flex-col h-full">
		<div
			bind:clientWidth={topbarWidth}
			class="flex items-center px-4 {condensedHeader ? 'h-9' : 'h-12'}"
		>
			<div class="flex gap-2 lg:gap-2 w-full items-center">
				<div class="flex flex-row items-center gap-2 min-w-0 shrink">
					<button
						disabled={customUi?.topBar?.settings == false}
						class="shrink-0"
						onclick={async () => {
							metadataOpen = true
						}}
					>
						<LanguageIcon lang={script.language} size={condensedHeader ? 18 : 24} />
					</button>
					{#if customUi?.topBar?.path != false}
						<div class="min-w-0 overflow-hidden">
							<EditorHeader
								bind:summary={script.summary}
								bind:path={script.path}
								savedPath={initialPath}
								kind="script"
								summaryEditable={customUi?.topBar?.editableSummary != false}
								pathEditable={customUi?.topBar?.editablePath != false}
								hidePath={condensedHeader}
								workspaceId={autosaveWorkspace}
								onNavigate={(item) => onNavigate?.(item)}
							/>
						</div>
					{/if}
					{#if opWorkspace}
						<AutosaveIndicator
							workspace={opWorkspace}
							itemKind="script"
							path={indicatorPath}
							draftOnly={savedScript?.no_deployed === true}
							{onResetToDeployed}
							{loadedFromDraft}
							{othersDraftsCount}
							{onOpenOthersDrafts}
						/>
					{/if}
				</div>

				<!-- Separator -->
				<div class="flex-1"></div>

				{#if $enterpriseLicense && initialPath != ''}
					<Awareness />
				{/if}

				<!-- Separator -->
				<div class="flex-1"></div>

				{#snippet settingsButton()}
					{#if customUi?.topBar?.settings != false}
						<Button
							aiId="script-builder-settings"
							aiDescription="Script builder settings to configure metadata, runtime, triggers, and generated UI."
							variant="default"
							unifiedSize={headerBtnSize}
							on:click={() => (metadataOpen = true)}
							startIcon={{ icon: Settings }}
							iconOnly={compactTopbar}
							title="Settings"
						>
							<span> Settings </span>
						</Button>
					{/if}
				{/snippet}
				{#snippet diffButton()}
					{#if customUi?.topBar?.diff != false}
						{@const isDraftOnly = savedScript?.no_deployed === true}
						{@const diffDisabled = !savedScript || !diffDrawer || isDraftOnly}
						{@const diffTitle = isDraftOnly
							? 'Deploy this script once to compare against the deployed version'
							: 'Diff'}
						<!-- A disabled <button> fires no pointer events, so a title/tooltip on it
						     never shows on hover. pointer-events-none on the button lets the hover
						     reach this titled wrapper instead. -->
						<div title={diffTitle} class={diffDisabled ? 'flex cursor-not-allowed' : 'flex'}>
							<Button
								variant="default"
								unifiedSize={headerBtnSize}
								on:click={() => openDiffDrawer()}
								disabled={diffDisabled}
								btnClasses={diffDisabled ? 'pointer-events-none' : undefined}
								iconOnly={compactTopbar}
								title={diffTitle}
								startIcon={{ icon: DiffIcon }}
							>
								Diff
							</Button>
						</div>
					{/if}
				{/snippet}
				{#if compactTopbar}
					<DropdownV2 items={getCompactMenuItems} placement="bottom-end">
						{#snippet buttonReplacement()}
							<Button
								nonCaptureEvent
								unifiedSize={headerBtnSize}
								variant="subtle"
								startIcon={{ icon: EllipsisVertical }}
								iconOnly
								title="More"
							/>
						{/snippet}
					</DropdownV2>
					{@render diffButton()}
					{@render settingsButton()}
				{:else}
					{@render diffButton()}
					{#if customUi?.topBar?.tagEdit != false}
						{#if scriptWorkerTags}
							{#if scriptWorkerTags?.length ?? 0 > 0}
								<div class="max-w-[200px]">
									<WorkerTagSelect
										nullTag={script.language}
										placeholder={customUi?.tagSelectPlaceholder}
										bind:tag={script.tag}
										size={headerBtnSize}
										workspaceId={opWorkspace}
									/>
								</div>
							{/if}
						{/if}
					{/if}
					{@render settingsButton()}
				{/if}

				<DeployButton
					loading={!fullyLoaded}
					{loadingSave}
					unifiedSize={headerBtnSize}
					dropdownItems={computeDropdownItems(initialPath, savedScript)}
					on:save={({ detail }) => handleEditScript(false, detail)}
				/>
			</div>
		</div>

		{#if showPipelineHint}
			<div
				class="flex items-center gap-2 px-4 py-1 border-y bg-surface-secondary text-xs text-secondary"
			>
				<Network size={14} class="shrink-0 text-tertiary" />
				<span class="truncate">
					This script can become a data pipeline step: annotate it with
					<code class="font-mono text-2xs bg-surface px-1 py-0.5 rounded border">-- pipeline</code>
					and
					<code class="font-mono text-2xs bg-surface px-1 py-0.5 rounded border">
						-- materialize
					</code>
					to materialize its result, or build it in the
					<a href="{base}/pipeline" class="text-blue-500 hover:underline">pipeline editor</a>.
					<a
						href="https://www.windmill.dev/docs/pipelines"
						target="_blank"
						rel="noreferrer"
						class="text-blue-500 hover:underline"
					>
						Learn more
					</a>
				</span>
				<div class="ml-auto shrink-0">
					<Button
						iconOnly
						variant="subtle"
						unifiedSize="sm"
						startIcon={{ icon: X }}
						title="Dismiss pipelines hint"
						onclick={() => (pipelineHintDismissed.val = true)}
					/>
				</div>
			</div>
		{/if}

		<ScriptEditor
			{disableAi}
			workspaceOverride={opWorkspace}
			sessionOpen={userDraftPath
				? {
						// URL draft path the editor loads/saves by, not the friendly
						// `script.path` (a new script's has no row → "not found").
						target: { kind: 'script', path: userDraftPath },
						workspaceId: opWorkspace ?? undefined,
						beforeOpen: persistDraftForSession
					}
				: undefined}
			bind:selectedTab={selectedInputTab}
			{customUi}
			{onTestJob}
			collabMode
			edit={initialPath != ''}
			on:format={() => {
				saveDraft()
			}}
			on:saveDraft={() => {
				saveDraft()
			}}
			on:openTriggers={openTriggers}
			on:applyArgs={applyArgs}
			on:addPreprocessor={addPreprocessor}
			bind:editor
			bind:this={scriptEditor}
			bind:schema={script.schema}
			path={script.path}
			stablePathForCaptures={initialPath || fakeInitialPath}
			bind:code={script.content}
			lang={script.language}
			timeout={script.timeout}
			kind={script.kind}
			autoKind={script.auto_kind}
			{template}
			tag={script.tag}
			lastSavedCode={savedScript?.content}
			lastDeployedCode={savedScript?.content}
			bind:args
			bind:hasPreprocessor
			bind:captureTable
			bind:assets={script.assets}
			bind:modules={script.modules}
			enablePreprocessorSnippet
			{testPanelCollapsed}
		/>
	</div>
{:else}
	Script Builder not available to operators
{/if}

<WacExportDrawer bind:this={wacExportDrawer} />
