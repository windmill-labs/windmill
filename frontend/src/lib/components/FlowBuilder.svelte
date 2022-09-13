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
	import { Breadcrumb, BreadcrumbItem } from 'flowbite-svelte'
	import { onDestroy, onMount, setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { OFFSET } from './CronInput.svelte'
	import FlowEditor from './flows/FlowEditor.svelte'
	import { flowStateStore, flowStateToFlow, type FlowState } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import { loadFlowSchedule, type Schedule } from './flows/scheduleUtils'
	import type { FlowEditorContext } from './flows/types'
	import { cleanInputs } from './flows/utils'

	import ScriptSchema from './ScriptSchema.svelte'

	export let initialPath: string = ''
	let pathError = ''

	$: step = Number($page.url.searchParams.get('step')) || 1

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
		const flow = cleanInputs(flowStateToFlow($flowStateStore, $flowStore))
		const { cron, args, enabled } = $scheduleStore

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

	async function changeStep(step: number) {
		goto(`?step=${step}`)
	}

	flowStateStore.subscribe((flowState: FlowState) => {
		if (flowState) {
			flowStore.update((flow: Flow) => {
				if (flow) {
					return flowStateToFlow(flowState, flow)
				}
				return flow
			})
		}
	})

	flowStore.subscribe((flow: Flow) => {
		if (flow) {
			setQueryWithoutLoad($page.url, 'state', encodeState(flow))
		}
	})

	const selectedIdStore = writable<string>('settings')
	const scheduleStore = writable<Schedule>(undefined)

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		select,
		path: initialPath
	})

	$: {
		loadFlowSchedule(initialPath, $workspaceStore)
			.then((schedule: Schedule) => {
				scheduleStore.set(schedule)
			})
			.catch(() => {
				scheduleStore.set({
					cron: '0 */5 * * *',
					args: {},
					enabled: false,
					previewArgs: {}
				})
			})
	}

	onMount(() => {
		loadHubScripts()
	})

	onDestroy(() => {
		//@ts-ignore
		$flowStore = undefined
		//@ts-ignore
		$flowStateStore = undefined
	})
</script>

<div class="flex flex-col flex-1 h-full">
	<!-- Nav between steps-->
	<div class="justify-between flex flex-row w-full my-4 px-4">
		<Breadcrumb>
			<BreadcrumbItem>
				<button on:click={() => changeStep(1)} class={step === 1 ? 'font-bold' : null}>
					Flow Editor
				</button>
			</BreadcrumbItem>
			<BreadcrumbItem>
				<button on:click={() => changeStep(2)} class={step === 2 ? 'font-bold' : null}>
					UI customisation
				</button>
			</BreadcrumbItem>
		</Breadcrumb>
		<div class="flex flex-row-reverse ml-2">
			{#if step == 1}
				<button
					disabled={pathError != ''}
					class="default-button px-6 max-h-8"
					on:click={() => changeStep(2)}
				>
					Next
				</button>
				<button
					disabled={pathError != ''}
					class="default-button-secondary px-6 max-h-8 mr-2"
					on:click={saveFlow}
				>
					Save
				</button>
			{:else}
				<button class="default-button px-6 self-end" on:click={saveFlow}>Save</button>
				<button
					class="default-button-secondary px-6 max-h-8 mr-2"
					on:click={async () => {
						changeStep(1)
					}}
				>
					Back
				</button>
			{/if}
		</div>
	</div>

	<!-- metadata -->

	{#if $flowStateStore}
		{#if step === 1}
			<FlowEditor />
		{:else if step === 2}
			<CenteredPage>
				<ScriptSchema
					synchronizedHeader={false}
					bind:summary={$flowStore.summary}
					bind:description={$flowStore.description}
					bind:schema={$flowStore.schema}
				/>
			</CenteredPage>
		{/if}
	{:else}
		<CenteredPage>Loading...</CenteredPage>
	{/if}
</div>
