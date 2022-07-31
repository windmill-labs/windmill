<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { FlowService, ScheduleService, type Flow } from '$lib/gen'
	import { clearPreviewResults, workspaceStore } from '$lib/stores'
	import {
		encodeState,
		formatCron,
		loadHubScripts,
		sendUserToast,
		setQueryWithoutLoad
	} from '$lib/utils'
	import { onMount } from 'svelte'
	import { OFFSET } from './CronInput.svelte'
	import FlowEditor from './FlowEditor.svelte'
	import { flowStore, mode } from './flows/flowStore'
	import { flowToMode } from './flows/utils'

	import ScriptSchema from './ScriptSchema.svelte'

	export let initialPath: string = ''
	let pathError = ''

	let scheduleArgs: Record<string, any>
	let scheduleEnabled
	let scheduleCron: string

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
		const flow = flowToMode($flowStore, $mode)

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
			if ($mode == 'pull') {
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

	flowStore.subscribe((flow: Flow) => {
		setQueryWithoutLoad($page.url, 'state', encodeState(flowToMode(flow, $mode)))
	})

	onMount(() => {
		loadHubScripts()
		clearPreviewResults()
	})
</script>

<div class="flex flex-col h-screen max-w-screen-lg xl:-ml-20 xl:pl-4 w-full -mt-4 pt-4 md:mx-10 ">
	<!-- Nav between steps-->
	<div class="flex flex-col w-full">
		<div class="justify-between flex flex-row drop-shadow-sm w-full">
			<div class="wizard-nav flex flex-row w-full">
				<button
					disabled={pathError != ''}
					class="{step === 1
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(1)
					}}>Step 1: Flow</button
				>
				<button
					disabled={pathError != ''}
					class="{step === 2
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(2)
					}}>Step 2: UI customisation</button
				>
			</div>
			<div class="flex flex-row-reverse ml-2">
				{#if step == 1}
					<button
						disabled={pathError != ''}
						class="default-button px-6 max-h-8"
						on:click={() => {
							changeStep(step + 1)
						}}
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
				{/if}
			</div>
		</div>
		<div class="flex flex-row-reverse">
			<span class="my-1 text-sm text-gray-500 italic">
				{#if initialPath && initialPath != $flowStore?.path} {initialPath} &rightarrow; {/if}
				{$flowStore?.path}
			</span>
		</div>
	</div>

	<!-- metadata -->

	{#if step === 1}
		<FlowEditor
			bind:pathError
			bind:initialPath
			bind:scheduleEnabled
			bind:scheduleCron
			bind:scheduleArgs
		/>
	{:else if step === 2}
		<ScriptSchema
			synchronizedHeader={false}
			bind:summary={$flowStore.summary}
			bind:description={$flowStore.description}
			bind:schema={$flowStore.schema}
		/>
	{/if}
</div>

<style>
	/* .wizard-nav {
		@apply w-1/2 sm:w-1/4;
	} */

	.wizard-nav button {
		max-height: 30px;
	}
</style>
