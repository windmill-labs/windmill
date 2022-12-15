<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { FlowService, ScheduleService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		encodeState,
		formatCron,
		loadHubScripts,
		sendUserToast,
		setQueryWithoutLoad
	} from '$lib/utils'
	import { faEye, faPen, faSave } from '@fortawesome/free-solid-svg-icons'
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { OFFSET } from './CronInput.svelte'
	import FlowGraphViewer from './FlowGraphViewer.svelte'
	import FlowEditor from './flows/FlowEditor.svelte'
	import { flowStateStore } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import FlowPreviewButtons from './flows/header/FlowPreviewButtons.svelte'
	import { loadFlowSchedule, type Schedule } from './flows/scheduleUtils'
	import type { FlowEditorContext } from './flows/types'
	import { cleanInputs } from './flows/utils'

	export let initialPath: string = ''
	export let selectedId: string | undefined
	export let initialArgs: Record<string, any> = {}
	export let loading = false

	let pathError = ''

	async function createSchedule(path: string) {
		const { cron, args, enabled } = $scheduleStore

		await ScheduleService.createSchedule({
			workspace: $workspaceStore!,
			requestBody: {
				path: path,
				schedule: formatCron(cron),
				offset: OFFSET,
				script_path: path,
				is_flow: true,
				args,
				enabled
			}
		})
	}

	async function saveFlow(): Promise<void> {
		const flow = cleanInputs($flowStore)
		const { cron, args, enabled } = $scheduleStore
		$dirtyStore = false

		if (initialPath === '') {
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
				path: initialPath
			})
			if (scheduleExists) {
				const schedule = await ScheduleService.getSchedule({
					workspace: $workspaceStore ?? '',
					path: initialPath
				})
				if (
					schedule.path != flow.path ||
					JSON.stringify(schedule.args) != JSON.stringify(args) ||
					schedule.schedule != cron
				) {
					await ScheduleService.updateSchedule({
						workspace: $workspaceStore ?? '',
						path: initialPath,
						requestBody: {
							schedule: formatCron(cron),
							script_path: flow.path,
							is_flow: true,
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
		sendUserToast(`Success! flow saved at ${$flowStore.path}`)
		goto(`/flows/get/${$flowStore.path}`)
	}

	let timeout: NodeJS.Timeout | undefined = undefined

	$: {
		if ($flowStore && $flowStateStore) {
			setUrl()
		}
	}

	function setUrl() {
		timeout && clearTimeout(timeout)
		timeout = setTimeout(
			() =>
				setQueryWithoutLoad(
					$page.url,
					'state',
					encodeState({
						flow: $flowStore,
						selectedId: $selectedIdStore
					})
				),
			500
		)
	}

	const selectedIdStore = writable<string>(selectedId)

	const scheduleStore = writable<Schedule>({ args: {}, cron: '', enabled: false })
	const previewArgsStore = writable<Record<string, any>>(initialArgs)

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		select,
		previewArgs: previewArgsStore
	})

	async function loadSchedule() {
		loadFlowSchedule(initialPath, $workspaceStore)
			.then((schedule: Schedule) => {
				scheduleStore.set(schedule)
			})
			.catch(() => {
				scheduleStore.set({
					cron: '0 */5 * * *',
					args: {},
					enabled: false
				})
			})
	}

	$: selectedId && select(selectedId)

	$: initialPath && $workspaceStore && loadSchedule()

	loadHubScripts()

	let flowViewer: Drawer
</script>

{#if !$userStore?.operator}
	<UnsavedConfirmationModal />

	<Drawer bind:this={flowViewer} size="75%">
		<DrawerContent title="View Graph" on:close={flowViewer.closeDrawer} noPadding>
			<div class="overflow-hidden h-full w-full">
				<FlowGraphViewer flow={$flowStore} />
			</div>
		</DrawerContent>
	</Drawer>

	<div class="flex flex-col flex-1 h-screen">
		<!-- Nav between steps-->
		<div
			class="justify-between flex flex-row w-full items-center pl-2.5 pr-6  space-x-4 overflow-x-auto scrollbar-hidden max-h-12 h-full"
		>
			<div class="flex flex-row">
				<FlowImportExportMenu />
				<Button
					btnClasses="inline-flex"
					startIcon={{ icon: faEye }}
					variant="border"
					color="light"
					size="sm"
					on:click={flowViewer.openDrawer}
				>
					View Graph
				</Button>
			</div>
			<div class="gap-1 flex-row hidden md:flex shrink overflow-hidden">
				<Button
					btnClasses="hidden lg:inline-flex"
					startIcon={{ icon: faPen }}
					variant="contained"
					color="light"
					size="xs"
					on:click={async () => {
						select('settings')
						document.getElementById('path')?.focus()
					}}
				>
					{$flowStore.path && $flowStore.path != '' ? $flowStore.path : 'Choose a path'}
				</Button>
				<Button
					startIcon={{ icon: faPen }}
					variant="contained"
					color="light"
					size="xs"
					on:click={async () => {
						select('settings')
						document.getElementById('flow-summary')?.focus()
					}}
				>
					<div class="max-w-[10em] !truncate">
						{$flowStore.summary == '' || !$flowStore.summary ? 'No summary' : $flowStore.summary}
					</div>
				</Button>
			</div>
			<div class="flex flex-row space-x-2">
				<FlowPreviewButtons />
				<div class="center-center">
					<Button
						disabled={pathError != ''}
						startIcon={{ icon: faSave }}
						size="sm"
						on:click={saveFlow}>Save</Button
					>
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
