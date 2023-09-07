<script lang="ts">
	import {
		FlowService,
		ScheduleService,
		type Flow,
		type FlowModule,
		DraftService,
		type PathScript,
		RawScript,
		ScriptService
	} from '$lib/gen'
	import { initHistory, redo, undo } from '$lib/history'
	import { enterpriseLicense, hubScripts, userStore, workspaceStore } from '$lib/stores'
	import { encodeState, formatCron, sleep } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import type { Drawer } from '$lib/components/common'

	import { faCalendarAlt, faSave } from '@fortawesome/free-solid-svg-icons'
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Badge, Button, Kbd, UndoRedo } from './common'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import FlowEditor from './flows/FlowEditor.svelte'
	import ScriptEditorDrawer from './flows/content/ScriptEditorDrawer.svelte'
	import type { FlowState } from './flows/flowState'
	import { dfs } from './flows/flowStore'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import { loadFlowSchedule, type Schedule } from './flows/scheduleUtils'
	import type { FlowEditorContext } from './flows/types'
	import { cleanInputs } from './flows/utils'
	import { Pen } from 'lucide-svelte'
	import { loadHubScripts } from '$lib/scripts'
	import { createEventDispatcher } from 'svelte'
	import Awareness from './Awareness.svelte'
	import { getAllModules } from './flows/flowExplorer'
	import {
		stepCopilot,
		type FlowCopilotModule,
		glueCopilot,
		type FlowCopilotContext
	} from './copilot/flow'
	import { numberToChars } from './flows/idUtils'
	import type { Schema, SchemaProperty } from '$lib/common'
	import FlowCopilotDrawer from './copilot/FlowCopilotDrawer.svelte'
	import FlowCopilotStatus from './copilot/FlowCopilotStatus.svelte'
	import { fade } from 'svelte/transition'

	export let initialPath: string = ''
	export let selectedId: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let loading = false
	export let flowStore: Writable<Flow>
	export let flowStateStore: Writable<FlowState>

	const dispatch = createEventDispatcher()

	async function createSchedule(path: string) {
		const { cron, timezone, args, enabled } = $scheduleStore

		try {
			await ScheduleService.createSchedule({
				workspace: $workspaceStore!,
				requestBody: {
					path: path,
					schedule: formatCron(cron),
					timezone,
					script_path: path,
					is_flow: true,
					args,
					enabled
				}
			})
		} catch (err) {
			sendUserToast(`The primary schedule could not be created: ${err}`, true)
		}
	}

	let loadingSave = false
	let loadingDraft = false

	async function saveDraft(): Promise<void> {
		loadingDraft = true
		try {
			const flow = cleanInputs($flowStore)

			$dirtyStore = false
			localStorage.removeItem('flow')
			localStorage.removeItem(`flow-${flow.path}`)

			if (initialPath == '') {
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: flow.path,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						tag: flow.tag,
						draft_only: true
					}
				})
			}
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: { path: initialPath == '' ? flow.path : initialPath, typ: 'flow', value: flow }
			})
			if (initialPath == '') {
				$dirtyStore = false
				dispatch('saveInitial')
			}
			sendUserToast('Saved as draft')
		} catch (error) {
			sendUserToast(`Error while saving the flow as a draft: ${error.body || error.message}`, true)
		}
		loadingDraft = false
	}

	export function computeUnlockedSteps(flow: Flow) {
		return Object.fromEntries(
			getAllModules(flow.value.modules, flow.value.failure_module)
				.filter((m) => m.value.type == 'script' && m.value.hash == null)
				.map((m) => [m.id, (m.value as PathScript).path])
		)
	}

	async function saveFlow(): Promise<void> {
		loadingSave = true
		try {
			const flow = cleanInputs($flowStore)
			// console.log('flow', computeUnlockedSteps(flow)) // del
			// loadingSave = false // del
			// return
			const { cron, timezone, args, enabled } = $scheduleStore
			$dirtyStore = false
			if (initialPath === '') {
				localStorage.removeItem('flow')
				localStorage.removeItem(`flow-${flow.path}`)
				await FlowService.createFlow({
					workspace: $workspaceStore!,
					requestBody: {
						path: flow.path,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema
					}
				})
				if (enabled) {
					await createSchedule(flow.path)
				}
			} else {
				localStorage.removeItem(`flow-${initialPath}`)
				await FlowService.updateFlow({
					workspace: $workspaceStore!,
					path: initialPath,
					requestBody: {
						path: flow.path,
						summary: flow.summary,
						description: flow.description ?? '',
						value: flow.value,
						schema: flow.schema,
						tag: flow.tag
					}
				})
				const scheduleExists = await ScheduleService.existsSchedule({
					workspace: $workspaceStore ?? '',
					path: flow.path
				})
				if (scheduleExists) {
					const schedule = await ScheduleService.getSchedule({
						workspace: $workspaceStore ?? '',
						path: flow.path
					})
					if (JSON.stringify(schedule.args) != JSON.stringify(args) || schedule.schedule != cron) {
						await ScheduleService.updateSchedule({
							workspace: $workspaceStore ?? '',
							path: flow.path,
							requestBody: {
								schedule: formatCron(cron),
								timezone,
								args
							}
						})
					}
					if (enabled != schedule.enabled) {
						await ScheduleService.setScheduleEnabled({
							workspace: $workspaceStore ?? '',
							path: flow.path,
							requestBody: { enabled }
						})
					}
				} else if (enabled) {
					await createSchedule(flow.path)
				}
			}
			loadingSave = false
			$dirtyStore = false
			dispatch('deploy')
		} catch (err) {
			sendUserToast(`The flow could not be saved: ${err.body}`, true)
			loadingSave = false
		}
	}

	let timeout: NodeJS.Timeout | undefined = undefined

	$: {
		if ($flowStore || $selectedIdStore) {
			saveSessionDraft()
		}
	}

	function saveSessionDraft() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(() => {
			try {
				localStorage.setItem(
					initialPath ? `flow-${initialPath}` : 'flow',
					encodeState({
						flow: $flowStore,
						selectedId: $selectedIdStore
					})
				)
			} catch (err) {
				console.error(err)
			}
		}, 500)
	}

	const selectedIdStore = writable<string>(selectedId ?? 'settings-metadata')

	export function getSelectedId() {
		return $selectedIdStore
	}

	const scheduleStore = writable<Schedule>({
		args: {},
		cron: '',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
		enabled: false
	})
	const previewArgsStore = writable<Record<string, any>>(initialArgs)
	const scriptEditorDrawer = writable<ScriptEditorDrawer | undefined>(undefined)
	const moving = writable<{ module: FlowModule; modules: FlowModule[] } | undefined>(undefined)
	const history = initHistory($flowStore)

	const testStepStore = writable<Record<string, any>>({})

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		previewArgs: previewArgsStore,
		scriptEditorDrawer,
		moving,
		history,
		flowStateStore,
		flowStore,
		testStepStore,
		saveDraft,
		initialPath
	})

	async function loadSchedule() {
		loadFlowSchedule(initialPath, $workspaceStore)
			.then((schedule: Schedule) => {
				scheduleStore.set(schedule)
			})
			.catch(() => {
				scheduleStore.set({
					cron: '0 */5 * * *',
					timezone: 'UTC',
					args: {},
					enabled: false
				})
			})
	}

	$: selectedId && select(selectedId)

	$: initialPath && $workspaceStore && loadSchedule()

	loadHubScripts()

	function onKeyDown(event: KeyboardEvent) {
		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}

		switch (event.key) {
			case 'Z':
				if (event.ctrlKey || event.metaKey) {
					$flowStore = redo(history)
					event.preventDefault()
				}
				break
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					$flowStore = undo(history, $flowStore)
					$selectedIdStore = 'Input'
					event.preventDefault()
				}
				break
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
					event.preventDefault()
				}
				break
			case 'ArrowDown': {
				let ids = generateIds()
				let idx = ids.indexOf($selectedIdStore)
				if (idx > -1 && idx < ids.length - 1) {
					$selectedIdStore = ids[idx + 1]
					event.preventDefault()
				}
				break
			}
			case 'ArrowUp': {
				let ids = generateIds()
				let idx = ids.indexOf($selectedIdStore)
				if (idx > 0 && idx < ids.length) {
					$selectedIdStore = ids[idx - 1]
					event.preventDefault()
				}
				break
			}
		}
	}

	function generateIds() {
		return [
			'settings-metadata',
			'constants',
			...dfs($flowStore.value.modules, (module) => module.id)
		]
	}

	const dropdownItems: Array<{
		label: string
		onClick: () => void
	}> = [
		{
			label: 'Exit & see details',
			onClick: () => dispatch('details')
		}
	]

	if (initialPath != '') {
		dropdownItems.push({
			label: 'Fork',
			onClick: () => window.open(`/flows/add?template=${initialPath}`)
		})
	}

	let flowCopilotContext: FlowCopilotContext = {
		drawerStore: writable<Drawer | undefined>(undefined),
		modulesStore: writable<FlowCopilotModule[]>([]),
		currentStepStore: writable<string | undefined>(undefined)
	}

	setContext('FlowCopilotContext', flowCopilotContext)

	const {
		drawerStore: copilotDrawerStore,
		modulesStore: copilotModulesStore,
		currentStepStore: copilotCurrentStepStore
	} = flowCopilotContext

	let doneTs = 0
	async function hubCompletions(text: string, idx: number, type: 'trigger' | 'script') {
		try {
			// make sure we display the results of the last request last
			const ts = Date.now()
			const scriptIds = await ScriptService.queryHubScripts({
				text: `${text}`,
				limit: 3,
				kind: type
			})
			if (ts < doneTs) return
			doneTs = ts

			const scripts = scriptIds
				.map((qs) => {
					const s = $hubScripts?.find((hs) => hs.ask_id === Number(qs.id))
					return s
				})
				.filter((s) => !!s)

			$copilotModulesStore[idx].hubCompletions = scripts as {
				path: string
				summary: string
				approved: boolean
				kind: string
				app: string
				ask_id: number
			}[]
		} catch (err) {
			if (err.name !== 'CancelError') throw err
		}
	}

	let abortController: AbortController | undefined = undefined
	let copilotLoading = false
	let flowCopilotMode: 'trigger' | 'sequence' = 'trigger'
	let copilotStatus: string = ''

	function getInitCopilotModules(mode: typeof flowCopilotMode): FlowCopilotModule[] {
		return [
			{
				id: 'a',
				type: mode === 'trigger' ? 'trigger' : 'script',
				description: '',
				code: '',
				hubCompletions: [],
				selectedCompletion: undefined,
				source: undefined
			},
			{
				id: 'b',
				type: 'script',
				description: '',
				code: '',
				hubCompletions: [],
				selectedCompletion: undefined,
				source: undefined
			}
		]
	}

	$: {
		copilotModulesStore.set(getInitCopilotModules(flowCopilotMode))
	}

	async function genFlow(i: number) {
		copilotLoading = true
		copilotStatus = "Generating code for step '" + numberToChars(i) + "'..."
		$copilotCurrentStepStore = numberToChars(i)
		try {
			abortController = new AbortController()

			$flowStore.value.modules = $flowStore.value.modules.slice(0, i)
			let prevCode = ''
			if (i === 0) {
				prevCode = ''
				$flowStore.schema = {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {},
					required: [],
					type: 'object'
				}
			} else {
				prevCode = ($flowStore.value.modules[i - 1].value as RawScript).content
			}

			let module = $copilotModulesStore[i]

			if (module.type === 'trigger') {
				if (!$scheduleStore.cron) {
					$scheduleStore.cron = '0 */15 * * *'
				}
				$scheduleStore.enabled = true
			}

			const flowModule = {
				id: numberToChars(i),
				stop_after_if:
					module.type === 'trigger'
						? {
								expr: 'result == undefined || Array.isArray(result) && result.length == 0',
								skip_if_stopped: true
						  }
						: undefined,
				value: {
					input_transforms: {},
					content: '',
					language: RawScript.language.BUN,
					type: 'rawscript' as const
				},
				summary:
					$copilotModulesStore[i].selectedCompletion?.summary ?? $copilotModulesStore[i].description
			}

			if (i === 1 && $copilotModulesStore[i - 1].type === 'trigger') {
				const loopModule: FlowModule = {
					id: numberToChars(i) + '_loop',
					value: {
						type: 'forloopflow',
						iterator: {
							type: 'javascript',
							expr: 'results.a'
						},
						skip_failures: true,
						modules: [flowModule]
					}
				}

				$flowStore.value.modules.push(loopModule)
			} else {
				$flowStore.value.modules.push(flowModule)
			}

			$copilotDrawerStore?.closeDrawer()
			select(numberToChars(i))
			await sleep(200)

			$copilotModulesStore[i].editor?.setCode('')
			const deltaStore = writable<string>('')
			const unsubscribe = deltaStore.subscribe(async (delta) => {
				$copilotModulesStore[i].editor?.append(delta)
			})
			await stepCopilot(module, deltaStore, prevCode, abortController)
			unsubscribe()

			copilotStatus = "Generating inputs for step '" + numberToChars(i) + "'..."
			await sleep(500) // make sure code was parsed

			try {
				let currentFlowModule = $flowStore.value.modules[i]
				if (currentFlowModule.value.type === 'forloopflow') {
					currentFlowModule = currentFlowModule.value.modules[0]
				}

				if (currentFlowModule.value.type === 'rawscript') {
					const stepSchema: Schema = JSON.parse(JSON.stringify($flowStateStore[module.id].schema)) // deep copy
					if (module.source === 'hub' && i >= 1) {
						// ask AI to set step inputs
						const pastModule = $flowStore.value.modules[i - 1]
						const inputs = await glueCopilot(
							Object.keys(currentFlowModule.value.input_transforms),
							pastModule.value.type === 'rawscript' ? pastModule.value.content : '',
							i === 1 && $copilotModulesStore[i - 1].type === 'trigger',
							abortController
						)

						// create flow inputs used by AI for autocompletion
						Object.entries(inputs)
							.filter(
								([key, expr]) =>
									key in stepSchema.properties &&
									expr.startsWith('flow_inputs.') &&
									!expr.startsWith('flow_inputs.iter')
							)
							.map(([key, _]) => {
								const inputSchemaProperty = stepSchema.properties[key]
								const isRequired = stepSchema.required.includes(key)

								if ($flowStore.schema) {
									$flowStore.schema.properties[key] = inputSchemaProperty
									if (isRequired) {
										$flowStore.schema.required.push(key)
									}
								} else {
									$flowStore.schema = {
										$schema: 'https://json-schema.org/draft/2020-12/schema',
										properties: {
											[key]: inputSchemaProperty
										},
										required: isRequired ? [key] : [],
										type: 'object'
									}
								}
								$flowStore.schema
							})

						flowModule.value.input_transforms = Object.entries(inputs).reduce(
							(acc, [key, expr]) => {
								acc[key] = {
									type: 'javascript',
									expr
								}
								return acc
							},
							{}
						)
					} else {
						// create possible flow inputs for autocompletion
						delete stepSchema.properties.prev_output
						$flowStore.schema = {
							$schema: 'https://json-schema.org/draft/2020-12/schema',
							properties: {
								...$flowStore.schema?.properties,
								...stepSchema.properties
							},
							required: Array.from(
								new Set([...$flowStore.schema?.required, ...stepSchema.required])
							),
							type: 'object'
						}

						// programatically set step inputs
						for (const key of Object.keys(currentFlowModule.value.input_transforms)) {
							if (key !== 'prev_output') {
								const schema = $flowStateStore[module.id].schema
								const schemaProperty = Object.entries(schema.properties).find(
									(x) => x[0] === key
								)?.[1]
								if (schemaProperty) {
									$flowStore.schema = {
										$schema: 'https://json-schema.org/draft/2020-12/schema',
										properties: {
											...$flowStore.schema?.properties,
											[key]: schemaProperty
										},
										required: schemaProperty.required
											? Array.from(new Set([...$flowStore.schema?.required, key]))
											: $flowStore.schema?.required,
										type: 'object'
									}
								}
							}

							flowModule.value.input_transforms[key] = {
								type: 'javascript',
								expr:
									key === 'prev_output'
										? $copilotModulesStore[i - 1].type === 'trigger'
											? 'flow_input.iter.value'
											: 'results.' + $copilotModulesStore[i - 1].id
										: 'flow_input.' + key
							}
						}
					}

					const wrappingFlowModule = $flowStore.value.modules[i]
					if (wrappingFlowModule.value.type === 'forloopflow') {
						wrappingFlowModule.value = {
							...wrappingFlowModule.value,
							modules: [flowModule]
						}
						$flowStore.value.modules[i] = wrappingFlowModule
					} else {
						$flowStore.value.modules[i] = flowModule
					}
				}
			} catch (err) {
				console.error(err)
			}

			copilotStatus =
				"Waiting for the user to validate code and inputs of step '" + numberToChars(i) + "'"
		} catch (err) {
			if (err?.message) {
				sendUserToast('Failed to generate code: ' + err.message, true)
			} else {
				sendUserToast('Failed to generate code', true)
				console.error(err)
			}
		} finally {
			copilotLoading = false
		}
	}

	async function handleFlowGenInputs() {
		copilotLoading = true
		select('Input')
		$copilotCurrentStepStore = 'Input'
		copilotStatus = 'Setting flow inputs...'

		// filter out unused flow inputs
		const flowInputs: Record<string, SchemaProperty> = {}
		const required = new Set<string>()
		function getFlowInputs(modules: FlowModule[]) {
			for (const module of modules) {
				if (module.value.type === 'rawscript') {
					for (const moduleAttr of Object.keys(module.value.input_transforms)) {
						const input = module.value.input_transforms[moduleAttr]
						if (
							input.type === 'javascript' &&
							input.expr.startsWith('flow_input.') &&
							!input.expr.startsWith('flow_input.iter')
						) {
							const flowAttr = input.expr.split('.')[1]
							const schema = $flowStateStore[module.id].schema
							const schemaProperty = Object.entries(schema.properties).find(
								(x) => x[0] === moduleAttr
							)?.[1]
							if (schemaProperty) {
								flowInputs[flowAttr] = schemaProperty
								required.add(flowAttr)
							}
						}
					}
				} else if (module.value.type === 'forloopflow') {
					getFlowInputs(module.value.modules)
				}
			}
		}
		getFlowInputs($flowStore.value.modules)

		$flowStore.schema = {
			$schema: 'https://json-schema.org/draft/2020-12/schema',
			properties: flowInputs,
			required: Array.from(required),
			type: 'object'
		}

		copilotStatus = "Done! Just check the flow's inputs and you're good to go!"
		$copilotCurrentStepStore = undefined
		copilotLoading = false
		await sleep(3000)
		copilotStatus = ''
	}

	function focusCopilot() {
		document.querySelectorAll('.splitpanes__splitter').forEach((el) => {
			el.classList.add('hidden')
		})
		document.querySelectorAll('#flow-graph *').forEach((el) => {
			if (el instanceof HTMLElement) {
				el.style.pointerEvents = 'none'
			}
		})
	}

	function blurCopilot() {
		document.querySelectorAll('.splitpanes__splitter').forEach((el) => {
			el.classList.remove('hidden')
		})
		document.querySelectorAll('#flow-graph *').forEach((el) => {
			if (el instanceof HTMLElement) {
				el.style.pointerEvents = ''
			}
		})
	}

	$: $copilotCurrentStepStore !== undefined ? focusCopilot() : blurCopilot()
</script>

<svelte:window on:keydown={onKeyDown} />

<FlowCopilotDrawer {hubCompletions} {genFlow} bind:flowCopilotMode />

{#if !$userStore?.operator}
	<ScriptEditorDrawer bind:this={$scriptEditorDrawer} />

	<div class="flex flex-col flex-1 h-screen">
		<!-- Nav between steps-->
		<div
			class="justify-between flex flex-row items-center pl-2.5 pr-6 space-x-4 scrollbar-hidden max-h-12 h-full relative"
		>
			{#if $copilotCurrentStepStore !== undefined}
				<div transition:fade class="absolute inset-0 bg-gray-500 bg-opacity-75 z-[900] !m-0" />
			{/if}
			<div class="flex w-full max-w-md gap-4 items-center">
				<div class="min-w-64 w-full">
					<input
						type="text"
						placeholder="Flow summary"
						class="text-sm w-full font-semibold"
						bind:value={$flowStore.summary}
					/>
				</div>
				<UndoRedo
					undoProps={{ disabled: $history.index === 0 }}
					redoProps={{ disabled: $history.index === $history.history.length - 1 }}
					on:undo={() => {
						$flowStore = undo(history, $flowStore)
						$selectedIdStore = 'Input'
					}}
					on:redo={() => {
						$flowStore = redo(history)
					}}
				/>
			</div>

			<div class="gap-4 flex-row hidden md:flex w-full max-w-md">
				{#if $scheduleStore.enabled}
					<Button
						btnClasses="hidden lg:inline-flex"
						startIcon={{ icon: faCalendarAlt }}
						variant="contained"
						color="light"
						size="xs"
						on:click={async () => {
							select('settings-schedule')
						}}
					>
						{$scheduleStore.cron ?? ''}
					</Button>
				{/if}
				<div class="flex justify-start w-full">
					<div>
						<button
							on:click={async () => {
								select('settings-metadata')
								document.getElementById('path')?.focus()
							}}
						>
							<Badge
								color="gray"
								class="center-center !bg-gray-300 !text-tertiary dark:!bg-gray-700 dark:!text-gray-300 !h-[28px]  !w-[70px] rounded-r-none"
							>
								<Pen size={12} class="mr-2" /> Path
							</Badge>
						</button>
					</div>
					<input
						type="text"
						readonly
						value={$flowStore.path && $flowStore.path != '' ? $flowStore.path : 'Choose a path'}
						class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none"
						on:focus={({ currentTarget }) => {
							currentTarget.select()
						}}
					/>
				</div>
			</div>
			<div class="flex flex-row space-x-2">
				{#if $enterpriseLicense && initialPath != ''}
					<Awareness />
				{/if}

				<FlowCopilotStatus
					{copilotLoading}
					bind:copilotStatus
					{genFlow}
					{handleFlowGenInputs}
					{abortController}
				/>

				<FlowImportExportMenu />

				<FlowPreviewButtons />
				<Button
					loading={loadingDraft}
					size="xs"
					startIcon={{ icon: faSave }}
					on:click={() => saveDraft()}
				>
					Save draft&nbsp;<Kbd small>Ctrl</Kbd><Kbd small>S</Kbd>
				</Button>
				<Button
					loading={loadingSave}
					size="xs"
					startIcon={{ icon: faSave }}
					on:click={() => saveFlow()}
					dropdownItems={initialPath != '' ? dropdownItems : undefined}
				>
					Deploy
				</Button>
			</div>
		</div>

		<!-- metadata -->
		{#if $flowStateStore}
			<FlowEditor {loading} />
		{:else}
			<CenteredPage>Loading...</CenteredPage>
		{/if}
	</div>
{:else}
	Flow Builder not available to operators
{/if}
