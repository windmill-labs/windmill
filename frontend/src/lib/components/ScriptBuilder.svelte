<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import {
		DraftService,
		type NewScript,
		ScriptService,
		type NewScriptWithDraft,
		type Script,
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
	import AIFormSettings from './copilot/AIFormSettings.svelte'
	import {
		defaultScripts,
		enterpriseLicense,
		usedTriggerKinds,
		userStore,
		workerTags,
		workspaceStore
	} from '$lib/stores'
	import {
		cleanValueProperties,
		emptySchema,
		emptyString,
		encodeState,
		generateRandomString,
		orderedJsonStringify,
		readFieldsRecursively,
		replaceFalseWithUndefined,
		type Value
	} from '$lib/utils'
	import Path from './Path.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import { Alert, Badge, Button, Drawer, SecondsInput, Tab, TabContent, Tabs } from './common'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage, Schema } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ErrorHandlerToggleButton from '$lib/components/details/ErrorHandlerToggleButton.svelte'
	import {
		Bug,
		Calendar,
		CheckCircle,
		Code,
		Pen,
		Plus,
		Rocket,
		Save,
		Settings,
		Shuffle,
		X
	} from 'lucide-svelte'
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
	import { onMount, setContext, untrack } from 'svelte'
	import Summary from './Summary.svelte'

	import DeployOverrideConfirmationModal from '$lib/components/common/confirmationModal/DeployOverrideConfirmationModal.svelte'
	import TriggersEditor from './triggers/TriggersEditor.svelte'
	import type { ScheduleTrigger, TriggerContext } from './triggers'
	import CaptureTable from './triggers/CaptureTable.svelte'
	import type { SavedAndModifiedValue } from './common/confirmationModal/unsavedTypes'
	import DeployButton from './DeployButton.svelte'
	import {
		type NewScriptWithDraftAndDraftTriggers,
		type Trigger,
		deployTriggers,
		filterDraftTriggers,
		handleSelectTriggerFromKind
	} from './triggers/utils'
	import DraftTriggersConfirmationModal from './common/confirmationModal/DraftTriggersConfirmationModal.svelte'
	import { Triggers } from './triggers/triggers.svelte'
	import type { ScriptBuilderProps } from './script_builder'
	import type { DiffDrawerI } from './diff_drawer'
	import WorkerTagSelect from './WorkerTagSelect.svelte'
	import { inputSizeClasses } from './text_input/TextInput.svelte'
	import type { ButtonType } from './common/button/model'
	import DebounceLimit from './flows/DebounceLimit.svelte'

	let {
		script = $bindable(),
		fullyLoaded = true,
		initialPath = $bindable(''),
		template = $bindable('script'),
		initialArgs = {},
		lockedLanguage = false,
		showMeta = false,
		neverShowMeta = false,
		diffDrawer = undefined,
		savedScript = $bindable(undefined),
		searchParams = new URLSearchParams(),
		disableHistoryChange = false,
		replaceStateFn = (url) => window.history.replaceState(null, '', url),
		customUi = {},
		savedPrimarySchedule = undefined,
		functionExports = undefined,
		children,
		onDeploy,
		onDeployError,
		onSaveInitial,
		onSeeDetails,
		onSaveDraftError,
		onSaveDraft,
		disableAi
	}: ScriptBuilderProps = $props()

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
	let open: boolean = $state(false) // Is confirmation modal open
	let args: Record<string, any> = $state(initialArgs) // Test args input
	let selectedInputTab: 'main' | 'preprocessor' = $state('main')
	let hasPreprocessor = $state(false)

	let metadataOpen = $state(
		!neverShowMeta &&
			(showMeta ||
				searchParams.get('metadata_open') == 'true' ||
				(initialPath == '' &&
					searchParams.get('state') == undefined &&
					searchParams.get('collab') == undefined))
	)

	let editor: Editor | undefined = $state(undefined)
	let scriptEditor: ScriptEditor | undefined = $state(undefined)
	let captureTable: CaptureTable | undefined = $state(undefined)

	// Draft triggers confirmation modal
	let draftTriggersModalOpen = $state(false)
	let confirmDeploymentCallback: (triggersToDeploy: Trigger[]) => void = () => {}

	async function handleDraftTriggersConfirmed(event: CustomEvent<{ selectedTriggers: Trigger[] }>) {
		const { selectedTriggers } = event.detail
		// Continue with saving the flow
		draftTriggersModalOpen = false
		confirmDeploymentCallback(selectedTriggers)
	}

	const primaryScheduleStore = writable<ScheduleTrigger | undefined | false>(savedPrimarySchedule) // keep for legacy
	const triggersCount = writable<TriggersCount | undefined>(
		savedPrimarySchedule
			? { schedule_count: 1, primary_schedule: { schedule: savedPrimarySchedule.cron } }
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
			workspace: $workspaceStore!,
			path: initialPath
		})

		await triggersState.fetchTriggers(
			triggersCount,
			$workspaceStore,
			initialPath,
			false,
			$primaryScheduleStore,
			$userStore
		)

		if (savedScript && savedScript.draft && savedScript.draft.draft_triggers) {
			savedScript = filterDraftTriggers(
				savedScript,
				triggersState
			) as NewScriptWithDraftAndDraftTriggers
		}
	}

	// Add triggers context store
	const triggersState = $state(
		new Triggers(
			[
				{ type: 'webhook', path: '', isDraft: false },
				{ type: 'default_email', path: '', isDraft: false },
				...(script.draft_triggers ?? [])
			],
			undefined,
			saveSessionDraft
		)
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
	let loadingDraft = $state(false)

	let timeout2: number | undefined = undefined
	function encodeScriptState(script: NewScript) {
		untrack(() => timeout2 && clearTimeout(timeout2))
		timeout2 = setTimeout(() => {
			replaceStateFn(
				'#' +
					encodeState({
						...script,
						draft_triggers: structuredClone(triggersState.getDraftTriggersSnapshot())
					})
			)
		}, 500)
	}

	let timeout: number | undefined = undefined
	function saveSessionDraft() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			encodeScriptState(script)
		}, 500)
	}

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	async function isTemplateScript() {
		const params = new URLSearchParams(window.location.search)
		const templateId = params.get('id')
		if (templateId === null) {
			return undefined
		}
		try {
			const templateScript = await PostgresTriggerService.getTemplateScript({
				workspace: $workspaceStore!,
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
		template: 'pgsql' | 'mysql' | 'script' | 'docker' | 'powershell' | 'bunnative'
	) {
		scriptEditor?.disableCollaboration()
		const templateScript = await isTemplateScript()
		script.content = initialCode(language, kind, template, templateScript != undefined)
		if (templateScript) {
			script.content += '\r\n' + templateScript
		}
		scriptEditor?.inferSchema(script.content, { nlang: language, resetArgs: true })
		if (script.content != editor?.getCode()) {
			setCode(script.content)
		}
	}

	async function handleEditScript(stay: boolean, deployMsg?: string): Promise<void> {
		// Fetch latest version and fetch entire script after if needed
		let actual_parent_hash: string | undefined = undefined

		try {
			if (initialPath && initialPath != '') {
				actual_parent_hash = (
					await ScriptService.getScriptLatestVersion({
						workspace: $workspaceStore!,
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
			workspace: $workspaceStore!,
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
			try {
				localStorage.removeItem(script.path)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			script.schema = script.schema ?? emptySchema()
			try {
				const result = await inferArgs(
					script.language,
					script.content,
					script.schema as any,
					script.kind === 'preprocessor' ? 'preprocessor' : undefined
				)
				if (script.kind === 'preprocessor') {
					script.no_main_func = undefined
					script.has_preprocessor = undefined
				} else {
					script.no_main_func = result?.no_main_func || undefined
					script.has_preprocessor = result?.has_preprocessor || undefined
				}
			} catch (error) {
				sendUserToast(`Could not parse code, are you sure it is valid?`, true)
			}

			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
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
					delete_after_use: script.delete_after_use,
					timeout: script.timeout,
					concurrency_key: emptyString(script.concurrency_key) ? undefined : script.concurrency_key,
					visible_to_runner_only: script.visible_to_runner_only,
					no_main_func: script.no_main_func,
					has_preprocessor: script.has_preprocessor,
					deployment_message: deploymentMsg || undefined,
					on_behalf_of_email: script.on_behalf_of_email,
					assets: script.assets
				}
			})

			if (!initialPath) {
				await CaptureService.moveCapturesAndConfigs({
					workspace: $workspaceStore!,
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
					$workspaceStore,
					!!$userStore?.is_admin || !!$userStore?.is_super_admin,
					usedTriggerKinds,
					script.path,
					true
				)
			}

			const { draft_triggers: _, ...newScript } = structuredClone($state.snapshot(script))
			savedScript = structuredClone($state.snapshot(newScript)) as NewScriptWithDraft
			setDraftTriggers([])

			if (!disableHistoryChange) {
				history.replaceState(history.state, '', `/scripts/edit/${script.path}`)
			}
			if (stay || (script.no_main_func && script.kind !== 'preprocessor')) {
				script.parent_hash = newHash
				sendUserToast('Deployed')
			} else {
				onDeploy?.({ path: script.path, hash: newHash })
			}
		} catch (error) {
			onDeployError?.({ path: script.path, error })
			sendUserToast(`Error while saving the script: ${error.body || error.message}`, true)
		}
		loadingSave = false
	}

	async function saveDraft(forceSave = false): Promise<void> {
		if (initialPath != '' && !savedScript) {
			return
		}

		if (savedScript) {
			const draftOrDeployed = cleanValueProperties(savedScript.draft || savedScript)
			const currentTriggers = structuredClone(triggersState.getDraftTriggersSnapshot())
			const current = cleanValueProperties({ ...script, draft_triggers: currentTriggers })
			if (!forceSave && orderedJsonStringify(draftOrDeployed) === orderedJsonStringify(current)) {
				sendUserToast('No changes detected, ignoring', false, [
					{
						label: 'Save anyway',
						callback: () => {
							saveDraft(true)
						}
					}
				])
				return
			}
		}

		loadingDraft = true
		try {
			try {
				localStorage.removeItem(script.path)
			} catch (e) {
				console.error('error interacting with local storage', e)
			}
			script.schema = script.schema ?? emptySchema()
			try {
				const result = await inferArgs(
					script.language,
					script.content,
					script.schema as any,
					script.kind === 'preprocessor' ? 'preprocessor' : undefined
				)
				if (script.kind === 'preprocessor') {
					script.no_main_func = undefined
					script.has_preprocessor = undefined
				} else {
					script.no_main_func = result?.no_main_func || undefined
					script.has_preprocessor = result?.has_preprocessor || undefined
				}
			} catch (error) {
				sendUserToast(`Could not parse code, are you sure it is valid?`, true)
			}
			let newHash = ''
			if (initialPath == '' || savedScript?.draft_only) {
				if (savedScript?.draft_only) {
					await ScriptService.deleteScriptByPath({
						workspace: $workspaceStore!,
						path: initialPath,
						keepCaptures: true
					})
					script.parent_hash = undefined
				}
				if (!initialPath || script.path != initialPath) {
					await CaptureService.moveCapturesAndConfigs({
						workspace: $workspaceStore!,
						path: initialPath || fakeInitialPath,
						requestBody: {
							new_path: script.path
						},
						runnableKind: 'script'
					})
				}
				newHash = await ScriptService.createScript({
					workspace: $workspaceStore!,
					requestBody: {
						path: script.path,
						summary: script.summary,
						description: script.description ?? '',
						content: script.content,
						schema: script.schema,
						is_template: script.is_template,
						language: script.language,
						kind: script.kind,
						tag: script.tag,
						draft_only: true,
						envs: script.envs,
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
						delete_after_use: script.delete_after_use,
						timeout: script.timeout,
						concurrency_key: emptyString(script.concurrency_key)
							? undefined
							: script.concurrency_key,
						visible_to_runner_only: script.visible_to_runner_only,
						no_main_func: script.no_main_func,
						has_preprocessor: script.has_preprocessor,
						on_behalf_of_email: script.on_behalf_of_email,
						assets: script.assets
					}
				})
			}
			const draftTriggers = triggersState.getDraftTriggersSnapshot()
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: initialPath == '' || savedScript?.draft_only ? script.path : initialPath,
					typ: 'script',
					value: {
						...script,
						draft_triggers: draftTriggers
					}
				}
			})

			const clonedScript = structuredClone($state.snapshot(script))
			savedScript = {
				...(initialPath == '' || savedScript?.draft_only
					? { ...clonedScript, draft_only: true }
					: savedScript),
				draft: {
					...clonedScript,
					draft_triggers: draftTriggers
				}
			} as NewScriptWithDraftAndDraftTriggers

			let savedAtNewPath = false
			if (initialPath == '' || (savedScript?.draft_only && script.path !== initialPath)) {
				savedAtNewPath = true
				initialPath = script.path
				onSaveInitial?.({ path: script.path, hash: newHash })
			}
			onSaveDraft?.({ path: script.path, savedAtNewPath, script })

			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(
				`Error while saving the script as a draft: ${error.body || error.message}`,
				true
			)
			onSaveDraftError?.({ path: script.path, error })
		}
		loadingDraft = false
	}

	function computeDropdownItems(
		initialPath: string,
		savedScript: NewScriptWithDraftAndDraftTriggers | undefined,
		diffDrawer: DiffDrawerI | undefined
	) {
		let dropdownItems: { label: string; onClick: () => void }[] =
			initialPath != '' && customUi?.topBar?.extraDeployOptions != false
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
						...(customUi?.topBar?.diff !== false && savedScript && diffDrawer
							? [
									{
										label: 'Show diff',
										onClick: async () => {
											if (!savedScript) {
												return
											}
											await syncWithDeployed()

											const currentDraftTriggers = structuredClone(
												triggersState.getDraftTriggersSnapshot()
											)

											const deployed = deployedValue ?? savedScript
											const current = { ...script, draft_triggers: currentDraftTriggers }
											if (current.assets && !current.assets.length) delete current.assets

											diffDrawer?.openDrawer()
											diffDrawer?.setDiff({
												mode: 'normal',
												deployed,
												draft: savedScript['draft'],
												current
											})
										}
									}
								]
							: []),
						...(!script.draft_only && script.kind === 'script' && !script.no_main_func
							? [
									{
										label: 'Exit & See details',
										onClick: () => {
											onSeeDetails?.({ path: initialPath })
										}
									}
								]
							: [])
					]
				: []

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
	let dirtyPath = $state(false)

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

	setContext('disableTooltips', customUi?.disableTooltips === true)

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

	function handleDeployTrigger(trigger: Trigger) {
		const { id, path, type } = trigger
		//Update the saved script to remove the draft trigger that is deployed
		if (savedScript && savedScript.draft && savedScript.draft.draft_triggers) {
			const newSavedDraftTrigers = savedScript.draft.draft_triggers.filter(
				(t) => t.id !== id || t.path !== path || t.type !== type
			)
			savedScript.draft.draft_triggers =
				newSavedDraftTrigers.length > 0 ? newSavedDraftTrigers : undefined
		}
	}

	function onScriptLanguageTrigger(lang: 'docker' | 'bunnative' | ScriptLang) {
		if (lang == 'docker') {
			if (isCloudHosted()) {
				sendUserToast(
					'You cannot use Docker scripts on the multi-tenant platform. Use a dedicated instance or self-host windmill instead.',
					true,
					[
						{
							label: 'Learn more',
							callback: () => {
								window.open('https://www.windmill.dev/docs/advanced/docker', '_blank')
							}
						}
					]
				)
				return
			}
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
	$effect(() => {
		readFieldsRecursively(script)
		!disableHistoryChange && encodeScriptState(script)
	})

	loadWorkerTags()
	async function loadWorkerTags() {
		if (!$workerTags) {
			$workerTags = await WorkerService.getCustomTagsForWorkspace({ workspace: $workspaceStore! })
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
												/>
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
														(lang == script.language && template == 'script') ||
														(template == 'bunnative' && lang == 'bunnative') ||
														(template == 'docker' && lang == 'docker')}
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
															{#if lang === 'ruby'}
																<span class="text-primary !text-xs"> BETA </span>
															{/if}
														</Button>
														{#snippet text()}
															{label} is only available with an enterprise license
														{/snippet}
													</Popover>
												{/each}
											</div>
										</Section>
									{/if}
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
														script.timeout = 300
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
												available on enterprise edition and for Python3, Deno, Bun and Bunnative. For other
												languages, the efficiency is already on par with dedicated workers since
												they do not spawn a full runtime</Tooltip
											>
										{/snippet}
									</Section>
									<Section label="Delete after use">
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
												once it is complete and the result has been returned.
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
												checked={Boolean(script.delete_after_use)}
												on:change={() => {
													if (script.delete_after_use) {
														script.delete_after_use = undefined
													} else {
														script.delete_after_use = true
													}
												}}
												options={{
													right: 'Delete logs, arguments and results after use'
												}}
											/>
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
									<Section label="On behalf of last editor">
										{#snippet header()}
											<Tooltip>
												When this option is enabled, the script will be run with the permissions of
												the last editor.
											</Tooltip>
										{/snippet}
										<div class="flex gap-2 shrink flex-col">
											<Toggle
												size="sm"
												checked={Boolean(script.on_behalf_of_email)}
												on:change={() => {
													if (script.on_behalf_of_email) {
														script.on_behalf_of_email = undefined
													} else {
														script.on_behalf_of_email = $userStore?.email
													}
												}}
												options={{
													right: 'Run on behalf of last editor'
												}}
											/>
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
									isDeployed={savedScript && !savedScript?.draft_only}
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

	<div class="flex flex-col h-screen">
		<div class="flex h-12 items-center px-4">
			<div class="flex gap-2 lg:gap-2 w-full items-center">
				<div class="flex flex-row gap-2 grow max-w-md">
					<div class="center-center">
						<button
							disabled={customUi?.topBar?.settings == false}
							onclick={async () => {
								metadataOpen = true
							}}
						>
							<LanguageIcon lang={script.language} size={24} />
						</button>
					</div>
					<Summary
						disabled={customUi?.topBar?.editableSummary == false}
						bind:value={script.summary}
					/>
				</div>

				<!-- Separator -->
				<div class="flex-1"></div>

				<div class="gap-4 flex whitespace-nowrap">
					{#if triggersState.triggers?.some((t) => t.type === 'schedule')}
						{@const primarySchedule = triggersState.triggers.findIndex((t) => t.isPrimary)}
						{@const schedule = triggersState.triggers.findIndex((t) => t.type === 'schedule')}

						<Button
							btnClasses="hidden lg:inline-flex"
							startIcon={{ icon: Calendar }}
							variant="contained"
							color="light"
							size="xs"
							on:click={async () => {
								metadataOpen = true
								selectedTab = 'triggers'
								triggersState.selectedTriggerIndex = primarySchedule ?? schedule
							}}
						>
							{triggersState.triggers[primarySchedule]?.draftConfig?.schedule ??
								triggersState.triggers[primarySchedule]?.lightConfig?.schedule ??
								''}
						</Button>
					{/if}
					{#if customUi?.topBar?.path != false}
						<div class="flex justify-start w-full items-center">
							{#if customUi?.topBar?.editablePath != false}
								<button
									onclick={async () => {
										metadataOpen = true
									}}
								>
									<Badge
										color="gray"
										class="center-center !bg-surface-secondary !text-primary {inputSizeClasses.md}  !w-[70px] rounded-r-none hover:!bg-surface-hover transition-all border border-r-0"
									>
										<Pen size={12} class="mr-2 shrink-0" /> Path
									</Badge>
								</button>
							{/if}
							<input
								type="text"
								readonly
								value={script.path}
								size={script.path?.length || 50}
								class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full {inputSizeClasses.md} !my-0 !py-0 !rounded-l-none border border-l-0 !shadow-none"
								onfocus={({ currentTarget }) => {
									currentTarget.select()
								}}
							/>
						</div>
					{/if}
				</div>

				{#if $enterpriseLicense && initialPath != ''}
					<Awareness />
				{/if}

				<!-- Separator -->
				<div class="flex-1"></div>

				{#if customUi?.topBar?.tagEdit != false}
					{#if $workerTags}
						{#if $workerTags?.length ?? 0 > 0}
							<div class="max-w-[200px]">
								<WorkerTagSelect
									nullTag={script.language}
									placeholder={customUi?.tagSelectPlaceholder}
									bind:tag={script.tag}
								/>
							</div>
						{/if}
					{/if}
				{/if}
				{#if customUi?.topBar?.settings != false}
					<Button
						aiId="script-builder-settings"
						aiDescription="Script builder settings to configure metadata, runtime, triggers, and generated UI."
						variant="default"
						unifiedSize="md"
						on:click={() => (metadataOpen = true)}
						startIcon={{ icon: Settings }}
					>
						<span class="hidden lg:flex"> Settings </span>
					</Button>
				{/if}
				<Button
					loading={loadingDraft}
					unifiedSize="md"
					variant="accent"
					startIcon={{ icon: Save }}
					on:click={() => saveDraft()}
					disabled={initialPath != '' && !savedScript}
					shortCut={{ key: 'S' }}
				>
					<span class="hidden lg:flex"> Draft </span>
				</Button>

				<DeployButton
					loading={!fullyLoaded}
					{loadingSave}
					newFlow={false}
					dropdownItems={computeDropdownItems(initialPath, savedScript, diffDrawer)}
					on:save={({ detail }) => handleEditScript(false, detail)}
				/>
			</div>
		</div>

		<ScriptEditor
			{disableAi}
			bind:selectedTab={selectedInputTab}
			{customUi}
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
			kind={script.kind}
			{template}
			tag={script.tag}
			lastSavedCode={savedScript?.draft?.content}
			lastDeployedCode={savedScript?.draft_only ? undefined : savedScript?.content}
			bind:args
			bind:hasPreprocessor
			bind:captureTable
			bind:assets={script.assets}
			enablePreprocessorSnippet
		/>
	</div>
{:else}
	Script Builder not available to operators
{/if}
