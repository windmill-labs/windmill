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
	import { faGlobe } from '@fortawesome/free-solid-svg-icons'
	import { Breadcrumb, BreadcrumbItem } from 'flowbite-svelte'
	import { onDestroy, onMount, setContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { writable } from 'svelte/store'
	import CenteredPage from './CenteredPage.svelte'
	import { OFFSET } from './CronInput.svelte'
	import FlowEditor from './flows/FlowEditor.svelte'
	import { flowStateStore } from './flows/flowState'
	import { flowStore } from './flows/flowStore'
	import FlowImportExportMenu from './flows/header/FlowImportExportMenu.svelte'
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
		const flow = cleanInputs($flowStore)
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

	flowStore.subscribe((flow: Flow) => {
		if (flow) {
			setQueryWithoutLoad($page.url, 'state', encodeState(flow))
		}
	})

	const selectedIdStore = writable<string>('settings')
	const scheduleStore = writable<Schedule>(undefined)
	const previewArgsStore = writable<Record<string, any>>(undefined)

	function select(selectedId: string) {
		selectedIdStore.set(selectedId)
	}

	setContext<FlowEditorContext>('FlowEditorContext', {
		selectedId: selectedIdStore,
		schedule: scheduleStore,
		select,
		path: initialPath,
		previewArgs: previewArgsStore
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
	<div class="justify-between flex flex-row w-full my-2 px-4">
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
		<div class="flex flex-row-reverse ml-2 space-x-reverse space-x-2">
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
					class="default-button-secondary px-6 max-h-8"
					on:click={saveFlow}
				>
					Save
				</button>
				<FlowImportExportMenu />

				<button
					class="text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-lg text-sm px-5 py-2.5 mr-2 mb-2 dark:bg-gray-800 dark:text-white dark:border-gray-600 dark:hover:bg-gray-700 dark:hover:border-gray-600 dark:focus:ring-gray-700"
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
				</button>
			{:else}
				<button class="default-button px-6 self-end" on:click={saveFlow}>Save</button>
				<button
					class="default-button-secondary px-6 max-h-8"
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
