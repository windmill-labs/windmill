<script lang="ts">
	import { goto } from '$app/navigation'
	import { FlowService, ScheduleService, type Flow, type FlowModule } from '$lib/gen'
	import { initHistory, redo, undo } from '$lib/history'
	import { userStore, workspaceStore } from '$lib/stores'
	import { encodeState, formatCron, loadHubScripts, sendUserToast } from '$lib/utils'
	import { faCalendarAlt, faSave } from '@fortawesome/free-solid-svg-icons'
	import { setContext } from 'svelte'
	import { writable, type Writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Badge, Button, UndoRedo } from './common'
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
	import Kbd from './common/kbd/Kbd.svelte'

	export let initialPath: string = ''
	export let selectedId: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let loading = false
	export let flowStore: Writable<Flow>
	export let flowStateStore: Writable<FlowState>

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

	async function saveFlow(leave: boolean): Promise<void> {
		loadingSave = true
		try {
			const flow = cleanInputs($flowStore)
			const { cron, timezone, args, enabled } = $scheduleStore
			$dirtyStore = false
			if (initialPath === '') {
				localStorage.removeItem('flow')
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
						schema: flow.schema
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
			if (leave) {
				goto(`/flows/get/${$flowStore.path}?workspace=${$workspaceStore}`)
			} else if (initialPath !== $flowStore.path) {
				initialPath = $flowStore.path
				goto(`/flows/edit/${$flowStore.path}?workspace=${$workspaceStore}`)
			}
		} catch (err) {
			sendUserToast(`The flow could not be saved: ${err.body}`, true)
			loadingSave = false
		}
	}

	let timeout: NodeJS.Timeout | undefined = undefined

	$: {
		if ($flowStore || $selectedIdStore) {
			saveDraft()
		}
	}

	function saveDraft() {
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
		testStepStore
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
					saveFlow(false)
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
			label: 'Save and exit',
			onClick: () => saveFlow(true)
		}
	]

	if (initialPath != '') {
		dropdownItems.push({
			label: 'Fork',
			onClick: () => window.open(`/flows/add?template=${initialPath}`)
		})
	}
</script>

<svelte:window on:keydown={onKeyDown} />

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
								class="center-center !bg-gray-300 !text-gray-600 !h-[28px]  !w-[70px] rounded-r-none"
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
				<FlowImportExportMenu />

				<FlowPreviewButtons />
				<div class="center-center">
					<Button
						title="Ctrl/Cmd + S"
						loading={loadingSave}
						size="xs"
						startIcon={{ icon: faSave }}
						on:click={() => saveFlow(false)}
						{dropdownItems}
					>
						Save
					</Button>
				</div>
			</div>
		</div>

		<!-- metadata -->
		{#if $flowStateStore}
			<FlowEditor {initialPath} {loading} />
		{:else}
			<CenteredPage>Loading...</CenteredPage>
		{/if}
	</div>
{:else}
	Flow Builder not available to operators
{/if}
