<script lang="ts">
	import {
		FlowService,
		ScheduleService,
		type Flow,
		type FlowModule,
		DraftService,
		type PathScript,
		RawScript,
		ScriptService,
		CancelablePromise
	} from '$lib/gen'
	import { initHistory, redo, undo } from '$lib/history'
	import { enterpriseLicense, hubScripts, userStore, workspaceStore } from '$lib/stores'
	import { capitalize, classNames, encodeState, formatCron, sleep } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import { Drawer, DrawerContent } from '$lib/components/common'

	import {
		faAdd,
		faCalendarAlt,
		faClose,
		faMagicWandSparkles,
		faSave
	} from '@fortawesome/free-solid-svg-icons'
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
	import { stepCopilot, type FlowCopilotModule, glueCopilot } from './copilot/flow'
	import { APP_TO_ICON_COMPONENT, WindmillIcon } from './icons'
	import { Icon } from 'svelte-awesome'
	import { numberToChars } from './flows/idUtils'
	import type { Schema, SchemaProperty } from '$lib/common'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ManualPopover from './ManualPopover.svelte'

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

	let aiDrawerStore = writable<Drawer | undefined>(undefined)

	setContext('ai-drawer', aiDrawerStore)

	let requests: { promise: CancelablePromise<{ id: string }[]>; ts: number }[] = []
	async function hubCompletions(text: string, idx: number, type: 'trigger' | 'script') {
		try {
			const currentRequest = ScriptService.queryHubScripts({
				text: `${text}`,
				limit: type === 'script' ? 3 : 10
			})

			// make sure we display the results of the last request last
			const ts = Date.now()
			requests.push({ promise: currentRequest, ts })
			const scriptIds = await currentRequest
			requests.forEach((r) => r.ts < ts && r.promise.cancel())
			requests = requests.filter((r) => r.ts <= ts)

			const scripts = scriptIds
				.map((qs) => {
					const s = $hubScripts
						?.filter((hs) => type === 'script' || hs.kind === 'trigger')
						.find((hs) => hs.ask_id === Number(qs.id))
					return s
				})
				.filter((s) => !!s)

			$flowCopilotModules[idx].hubCompletions = scripts.slice(0, 3) as {
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
	let waitingStep: number | undefined = undefined
	let flowCopilotMode: 'trigger' | 'sequence' = 'trigger'
	let copilotPopover: ManualPopover | undefined = undefined
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

	let flowCopilotModules: Writable<FlowCopilotModule[]> = writable(
		getInitCopilotModules(flowCopilotMode)
	)

	$: {
		flowCopilotModules.set(getInitCopilotModules(flowCopilotMode))
	}

	setContext('flow-copilot-modules', flowCopilotModules)

	$: copilotStatus.length > 0 ? copilotPopover?.open() : copilotPopover?.close()

	async function genFlow(i: number) {
		waitingStep = undefined
		copilotLoading = true
		copilotStatus = "Generating code for step '" + numberToChars(i) + "'..."
		try {
			abortController = new AbortController()

			let prevCode = ''
			if (i === 0) {
				prevCode = ''
				$flowStore.value.modules = []
				$flowStore.schema = {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {},
					required: [],
					type: 'object'
				}
			} else {
				prevCode = ($flowStore.value.modules[i - 1].value as RawScript).content
			}

			let module = $flowCopilotModules[i]
			const flowModule = {
				id: numberToChars(i),
				value: {
					input_transforms: {},
					content: '',
					language: RawScript.language.DENO,
					type: 'rawscript' as const
				}
			}

			if (i === 1 && $flowCopilotModules[i - 1].type === 'trigger') {
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

			$aiDrawerStore?.closeDrawer()
			select(numberToChars(i))
			await sleep(200)

			const deltaStore = writable<string>('')
			const unsubscribe = deltaStore.subscribe(async (delta) => {
				// $flowCopilotModules[i].editor?.setCode(code)
				$flowCopilotModules[i].editor?.append(delta)
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
							i === 1 && $flowCopilotModules[i - 1].type === 'trigger',
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
								expr: key === 'prev_output' ? 'flow_input.iter.value' : 'flow_input.' + key
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
			waitingStep = i
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
		waitingStep = undefined

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
		copilotLoading = false
		await sleep(3000)
		copilotStatus = ''
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<Drawer bind:this={$aiDrawerStore}>
	<DrawerContent on:close={$aiDrawerStore.closeDrawer}>
		<h1 class="pb-4">AI Flow Builder</h1>
		<div class="flex flex-col gap-4">
			<ToggleButtonGroup bind:selected={flowCopilotMode}>
				<ToggleButton value="trigger" label="Trigger" />
				<ToggleButton value="sequence" label="Sequence" />
			</ToggleButtonGroup>
			{#each $flowCopilotModules as copilotModule, i}
				<div>
					{#if i === 1 && $flowCopilotModules[i - 1].type === 'trigger'}
						<div class="flex flex-row items-center pb-2 gap-2">
							<Badge color="indigo">{numberToChars(i)}_loop</Badge>
							<p class="font-semibold">For loop</p>
						</div>
					{/if}
					<div class={i === 1 && $flowCopilotModules[i - 1].type === 'trigger' ? 'pl-4' : ''}>
						<div class="flex flex-row items-center justify-between">
							<div class="flex flex-row items-center gap-2">
								<Badge color="indigo">{numberToChars(i)}</Badge>
								<p class="font-semibold"
									>{copilotModule.type === 'trigger' ? 'Trigger' : 'Action'}</p
								>
							</div>
							{#if flowCopilotMode === 'sequence' && i >= 1}
								<button
									on:click={() => {
										flowCopilotModules.update((prev) => {
											prev.splice(i, 1)
											return prev
										})
									}}
								>
									<Icon data={faClose} />
								</button>
							{/if}
						</div>
						{#if copilotModule.source !== undefined}
							<div
								class="p-4 gap-4 flex flex-row grow bg-surface transition-all items-center rounded-md justify-between border"
							>
								<div class="flex items-center gap-4">
									<div
										class={classNames(
											'rounded-md p-1 flex justify-center items-center border',
											'bg-surface border'
										)}
									>
										{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion}
											<svelte:component
												this={APP_TO_ICON_COMPONENT[copilotModule.selectedCompletion['app']]}
												height={18}
												width={18}
											/>
										{:else}
											<Icon data={faMagicWandSparkles} />
										{/if}
									</div>

									<div class="w-full text-left font-normal">
										<div class="text-primary flex-wrap text-md font-semibold mb-1">
											{copilotModule.source === 'hub' && copilotModule.selectedCompletion
												? copilotModule.selectedCompletion.summary
												: copilotModule.description}
										</div>
										{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion}
											<div class="text-secondary text-xs break-all">
												{copilotModule.selectedCompletion.path}
											</div>
										{/if}
									</div>
								</div>
								{#if copilotModule.source === 'hub' && copilotModule.selectedCompletion && copilotModule.selectedCompletion?.kind !== 'script'}
									<Badge color="gray" baseClass="border"
										>{capitalize(copilotModule.selectedCompletion.kind)}</Badge
									>
								{/if}

								<button
									on:click={() => {
										copilotModule.selectedCompletion = undefined
										copilotModule.source = undefined
									}}
								>
									<Icon data={faClose} />
								</button>
							</div>
						{:else}
							<input
								name="description"
								type="text"
								placeholder={copilotModule.type === 'trigger'
									? 'describe what should trigger your flow'
									: 'describe what this step should do'}
								bind:value={copilotModule.description}
								on:input={() => {
									if (copilotModule.description.length > 2) {
										hubCompletions(copilotModule.description, i, copilotModule.type)
									} else {
										copilotModule.hubCompletions = []
									}
								}}
							/>
						{/if}
						{#if copilotModule.description.length > 3 && copilotModule.source === undefined}
							<ul class="divide-y border rounded-md transition-all mt-2">
								<li>
									<button
										class="p-4 gap-4 flex flex-row hover:bg-surface-hover bg-surface transition-all items-center rounded-md justify-between w-full"
										on:click={() => {
											copilotModule.source = 'custom'
											copilotModule.selectedCompletion = undefined
										}}
									>
										<div class="flex items-center gap-4">
											<div
												class={classNames(
													'rounded-md p-1 flex justify-center items-center border',
													'bg-surface border'
												)}
											>
												<Icon data={faMagicWandSparkles} />
											</div>

											<div class="text-left font-normal">
												<div class="text-primary flex-wrap text-md font-semibold mb-1">
													Generate step from scratch using Windmill AI
												</div>
											</div>
										</div>
									</button>
								</li>
								{#each copilotModule.hubCompletions as item (item.path)}
									<li>
										<button
											class="p-4 gap-4 flex flex-row hover:bg-surface-hover bg-surface transition-all items-center rounded-md justify-between w-full"
											on:click={() => {
												copilotModule.source = 'hub'
												copilotModule.selectedCompletion = item
											}}
										>
											<div class="flex items-center gap-4">
												<div
													class={classNames(
														'rounded-md p-1 flex justify-center items-center border',
														'bg-surface border'
													)}
												>
													<svelte:component
														this={APP_TO_ICON_COMPONENT[item['app']]}
														height={18}
														width={18}
													/>
												</div>

												<div class="text-left font-normal">
													<div class="text-primary text-md font-semibold mb-1">
														{item.summary ?? ''}
													</div>
													<div class="text-secondary text-xs break-all">
														{item.path}
													</div>
												</div>
											</div>
											{#if item.kind !== 'script'}
												<Badge color="gray" baseClass="border">{capitalize(item.kind)}</Badge>
											{/if}
										</button>
									</li>
								{/each}
							</ul>
						{/if}
					</div>
				</div>
			{/each}
			{#if flowCopilotMode !== 'trigger'}
				<div class="flex justify-start">
					<Button
						startIcon={{ icon: faAdd }}
						size="xs"
						variant="border"
						on:click={() =>
							flowCopilotModules.update((prev) => [
								...prev,
								{
									id: numberToChars(prev.length),
									type: 'script',
									description: '',
									code: '',
									source: undefined,
									hubCompletions: [],
									selectedCompletion: undefined
								}
							])}>Add step</Button
					>
				</div>
			{/if}

			<Button
				on:click={() => genFlow(0)}
				spacingSize="md"
				startIcon={{ icon: faMagicWandSparkles }}
				disabled={$flowCopilotModules.find((m) => m.source === undefined) !== undefined}
			>
				Build flow
			</Button>
		</div>
	</DrawerContent>
</Drawer>

{#if !$userStore?.operator}
	<ScriptEditorDrawer bind:this={$scriptEditorDrawer} />

	<div class="flex flex-col flex-1 h-screen">
		<!-- Nav between steps-->
		<div
			class="justify-between flex flex-row items-center pl-2.5 pr-6 space-x-4 scrollbar-hidden max-h-12 h-full"
		>
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

				<ManualPopover bind:this={copilotPopover}>
					<Button
						size="xs"
						btnClasses="mr-2"
						on:click={() => {
							if (copilotLoading || waitingStep !== undefined) {
								abortController?.abort()
								waitingStep = undefined
								copilotStatus = ''
							} else {
								$aiDrawerStore?.openDrawer()
							}
						}}
						startIcon={copilotLoading
							? undefined
							: {
									icon: faMagicWandSparkles
							  }}
						color={copilotLoading || waitingStep !== undefined ? 'red' : 'light'}
						variant="border"
					>
						{#if copilotLoading}
							<WindmillIcon class="mr-1 text-white" height="16px" width="20px" spin="veryfast" />
						{/if}

						{copilotLoading || waitingStep !== undefined ? 'Cancel' : 'AI Flow Builder'}
					</Button>
					<div slot="content" class="text-sm flex flex-row items-center"
						><span class="font-semibold">{copilotStatus}</span>
						{#if waitingStep !== undefined}
							<Button
								btnClasses="ml-2"
								color="green"
								on:click={() => {
									if (waitingStep === undefined) {
										return
									}
									if (waitingStep >= $flowCopilotModules.length - 1) {
										handleFlowGenInputs()
									} else {
										genFlow(waitingStep + 1)
									}
								}}
							>
								Validate and continue
							</Button>
						{/if}</div
					>
				</ManualPopover>

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
