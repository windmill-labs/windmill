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
	import { Breadcrumb, BreadcrumbItem } from 'flowbite-svelte'
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

	flowStore.subscribe((flow: Flow) => {
		setQueryWithoutLoad($page.url, 'state', encodeState(flowToMode(flow, $mode)))
	})

	onMount(() => {
		loadHubScripts()
		clearPreviewResults()
	})
</script>

<div class="flex flex-col max-w-screen-lg w-full mb-96">
	<!-- Nav between steps-->
	<div class="justify-between flex flex-row w-full">
		<Breadcrumb>
			<BreadcrumbItem href={step != 1 && '?step=1'} variation={step === 1 ? 'solid' : null}>
				Flow Editor
			</BreadcrumbItem>
			<BreadcrumbItem href={step != 2 && '?step=2'} variation={step === 2 ? 'solid' : null}>
				UI customisation
			</BreadcrumbItem>
		</Breadcrumb>
		<div class="flex flex-row-reverse ml-2">
			{#if step == 1}
				<button
					disabled={pathError != ''}
					class="default-button px-6 max-h-8"
					href={`?step=${step + 1}`}
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
