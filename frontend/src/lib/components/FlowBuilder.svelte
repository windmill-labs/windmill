<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { FlowService, ScheduleService, type Flow } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import {
		encodeState,
		formatCron,
		loadHubScripts,
		pathIsEmpty,
		sendUserToast,
		setQueryWithoutLoad
	} from '$lib/utils'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Breadcrumb, BreadcrumbItem, Button } from 'flowbite-svelte'
	import { onDestroy, onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import { OFFSET } from './CronInput.svelte'
	import Drawer from './Drawer.svelte'
	import FlowEditor from './FlowEditor.svelte'
	import FlowPreviewContent from './FlowPreviewContent.svelte'
	import { flowStateStore, flowStateToFlow, type FlowState } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import { cleanInputs } from './flows/utils'

	import ScriptSchema from './ScriptSchema.svelte'

	export let initialPath: string = ''
	let pathError = ''

	let previewOpen = false

	let scheduleArgs: Record<string, any> = {}
	let previewArgs: Record<string, any> = {}
	let scheduleEnabled: boolean = false
	let scheduleCron: string = ''

	$: step = Number($page.url.searchParams.get('step')) || 1

	async function createSchedule(path: string) {
		await ScheduleService.createSchedule({
			workspace: $workspaceStore!,
			requestBody: {
				path: path,
				schedule: formatCron(scheduleCron),
				offset: OFFSET,
				script_path: path,
				is_flow: true,
				args: scheduleArgs,
				enabled: scheduleEnabled
			}
		})
	}

	async function saveFlow(): Promise<void> {
		const flow = cleanInputs(flowStateToFlow($flowStateStore, $flowStore))

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
			if (scheduleEnabled) {
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
					JSON.stringify(schedule.args) != JSON.stringify(scheduleArgs) ||
					schedule.schedule != scheduleCron
				) {
					await ScheduleService.updateSchedule({
						workspace: $workspaceStore ?? '',
						path: initialPath,
						requestBody: {
							schedule: formatCron(scheduleCron),
							script_path: flow.path,
							is_flow: true,
							args: scheduleArgs
						}
					})
				}
				if (scheduleEnabled != schedule.enabled) {
					await ScheduleService.setScheduleEnabled({
						workspace: $workspaceStore ?? '',
						path: flow.path,
						requestBody: { enabled: scheduleEnabled }
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

<div class="flex flex-row w-full h-full justify-between ">
	<div class={`flex flex-col mb-96 m-auto w-full`}>
		<!-- Nav between steps-->
		<div class="justify-between flex flex-row w-full my-4">
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
						}}>Back</button
					>
				{/if}
			</div>
		</div>
		<div class="flex flex-row-reverse">
			<span class="my-1 text-sm text-gray-500 italic">
				{#if initialPath && initialPath != $flowStore?.path} {initialPath} &rightarrow; {/if}
				{$flowStore?.path}
			</span>
		</div>

		<!-- metadata -->

		{#if $flowStateStore}
			{#if step === 1}
				<FlowEditor
					bind:pathError
					bind:initialPath
					bind:scheduleEnabled
					bind:scheduleCron
					bind:scheduleArgs
					bind:previewArgs
				/>
				<Button
					disabled={pathIsEmpty($flowStore.path)}
					size="lg"
					pill
					on:click={() => (previewOpen = !previewOpen)}
					class={`blue-button fixed bottom-10 right-10 ${previewOpen ? 'hidden' : ''}`}
				>
					Preview flow
					{pathIsEmpty($flowStore.path) ? '(pick a name first!)' : ''}

					<Icon data={faPlay} class="ml-2" />
				</Button>
			{:else if step === 2}
				<ScriptSchema
					synchronizedHeader={false}
					bind:summary={$flowStore.summary}
					bind:description={$flowStore.description}
					bind:schema={$flowStore.schema}
				/>
			{/if}
		{:else}
			<p>Loading</p>
		{/if}
	</div>
</div>

{#if $flowStateStore && $flowStore}
	<Drawer bind:open={previewOpen} size="800px">
		<FlowPreviewContent bind:args={previewArgs} on:close={() => (previewOpen = !previewOpen)} />
	</Drawer>
{/if}
