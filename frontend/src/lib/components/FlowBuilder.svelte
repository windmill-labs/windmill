<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { FlowService, ScheduleService, type Flow } from '$lib/gen'
	import { clearPreviewResults, previewResults, workspaceStore } from '$lib/stores'
	import {
		encodeState,
		formatCron,
		loadHubScripts,
		sendUserToast,
		setQueryWithoutLoad
	} from '$lib/utils'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'
	import { Breadcrumb, BreadcrumbItem, Button } from 'flowbite-svelte'
	import { onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import { OFFSET } from './CronInput.svelte'
	import FlowEditor from './FlowEditor.svelte'
	import FlowPreviewContent from './FlowPreviewContent.svelte'
	import { flowStore, mode } from './flows/flowStore'
	import { flowToMode, jobsToResults } from './flows/utils'

	import ScriptSchema from './ScriptSchema.svelte'

	export let initialPath: string = ''
	let pathError = ''

	let scheduleArgs: Record<string, any>
	let scheduleEnabled
	let scheduleCron: string

	let previewOpen = false

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
		if (flow) {
			setQueryWithoutLoad($page.url, 'state', encodeState(flowToMode(flow, $mode)))
		}
	})

	onMount(() => {
		loadHubScripts()
		clearPreviewResults()
	})
</script>

<div class="flex flex-row w-full h-full justify-between">
	<div class="flex flex-col max-w-screen-md mb-96 m-auto">
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

		{#if $flowStore}
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
		{:else}
			<p>Loading</p>
		{/if}
	</div>

	<Button
		size="lg"
		pill
		on:click={() => (previewOpen = !previewOpen)}
		class={`blue-button fixed bottom-10 right-10 ${previewOpen ? 'hidden' : ''}`}
	>
		Preview flow
		<Icon data={faPlay} class="ml-2" />
	</Button>

	<div class={`relative h-screen w-1/3 ${previewOpen ? '' : 'hidden'}`}>
		<div class="absolute top-0 h-full">
			<div class="fixed border-l-2 right-0 h-screen w-1/3">
				<FlowPreviewContent
					flow={$flowStore}
					mode={$mode}
					bind:args={scheduleArgs}
					on:close={() => (previewOpen = !previewOpen)}
					on:change={(e) => {
						previewResults.set(jobsToResults(e.detail))
					}}
				/>
			</div>
		</div>
	</div>
</div>
