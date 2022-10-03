<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { FlowService, ScheduleService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		encodeState,
		formatCron,
		loadHubScripts,
		sendUserToast,
		setQueryWithoutLoad
	} from '$lib/utils'
	import { faGlobe, faPen } from '@fortawesome/free-solid-svg-icons'
	import { setContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import Button from './common/button/Button.svelte'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { OFFSET } from './CronInput.svelte'
	import FlowEditor from './flows/FlowEditor.svelte'
	import { flowStateStore } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
	import { loadFlowSchedule, type Schedule } from './flows/scheduleUtils'
	import type { FlowEditorContext } from './flows/types'
	import { cleanInputs } from './flows/utils'

	export let initialPath: string = ''
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
			} else {
				await createSchedule(flow.path)
			}
		}
		sendUserToast(`Success! flow saved at ${$flowStore.path}`)
		goto(`/flows/get/${$flowStore.path}`)
	}

	flowStore.subscribe((flow: Flow) => {
		if (flow) {
			setQueryWithoutLoad($page.url, 'state', encodeState(flow))
		}
	})

	const selectedIdStore = writable<string>('settings')
	const scheduleStore = writable<Schedule>({ args: {}, cron: '', enabled: false })
	const previewArgsStore = writable<Record<string, any>>({})

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

	$: initialPath && $workspaceStore && loadSchedule()

	loadHubScripts()
</script>

<UnsavedConfirmationModal />

<div class="flex flex-col flex-1 h-full">
	<!-- Nav between steps-->
	<div class="justify-between flex flex-row w-full my-2 px-4 space-x-4 h-10">
		<div id="flow_title" class="flex justify-between items-center">
			<button class="flex flex-row items-center w-full h-full" on:click={() => select('settings')}>
				<span class="font-mono text-sm"> {$flowStore.path}</span>
				<Icon
					data={faPen}
					scale={0.8}
					class="text-gray-500 ml-2 flex justify-center items-center mb-0.5"
				/>
			</button>
		</div>
		<div class="shrink h-full">
			<button
				class="flex flex-row items-center w-full h-full"
				on:click={() => {
					select('settings')
					document.getElementById('flow-summary')?.focus()
				}}
			>
				<div class="overflow-x-auto flex items-center h-full text-sm text-left font-semibold">
					<div>{$flowStore.summary ?? ''}</div>
				</div>
				<div>
					<Icon data={faPen} scale={0.8} class="text-gray-500 ml-1" />
				</div>
			</button>
		</div>
		<div class="flex flex-row-reverse ml-2 space-x-reverse space-x-2">
			<Button disabled={pathError != ''} color="blue" size="sm" on:click={saveFlow}>Save</Button>
			<FlowImportExportMenu />

			<Button
				color="light"
				size="sm"
				variant="border"
				on:click={() => {
					const url = new URL('https://hub.windmill.dev/flows/add')
					const openFlow = {
						value: $flowStore.value,
						summary: $flowStore.summary,
						description: $flowStore.description,
						schema: $flowStore.schema
					}
					url.searchParams.append('flow', btoa(JSON.stringify(openFlow)))
					window.open(url, '_blank')?.focus()
				}}
			>
				<Icon data={faGlobe} scale={0.8} class="inline mr-2" />
				Publish to Hub
			</Button>
		</div>
	</div>

	<!-- metadata -->

	{#if $flowStateStore}
		<FlowEditor {initialPath} />
	{:else}
		<CenteredPage>Loading...</CenteredPage>
	{/if}
</div>
