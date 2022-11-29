<script lang="ts">
	import { Job, JobService, type Flow, type FlowModule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faClose, faPlay, faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { Button, Kbd } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import { dfs, flowStore } from './flows/flowStore'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import { flowStateStore } from './flows/flowState'
	import CapturePayload from './flows/content/CapturePayload.svelte'

	let capturePayload: CapturePayload
	export let previewMode: 'upTo' | 'whole'
	export let open: boolean

	let jobId: string | undefined = undefined
	let job: Job | undefined = undefined
	let isValid: boolean = false
	let isRunning: boolean = false
	let jobProgressReset: () => void

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	function sliceModules(modules: FlowModule[], upTo: number, idOrders: string[]): FlowModule[] {
		return modules
			.filter((x) => idOrders.indexOf(x.id) <= upTo)
			.map((m) => {
				if (m.value.type === 'forloopflow') {
					m.value.modules = sliceModules(m.value.modules, upTo, idOrders)
				} else if (m.value.type === 'branchone') {
					m.value.branches = m.value.branches.map((b) => {
						b.modules = sliceModules(b.modules, upTo, idOrders)
						return b
					})
					m.value.default = sliceModules(m.value.default, upTo, idOrders)
				} else if (m.value.type === 'branchall') {
					m.value.branches = m.value.branches.map((b) => {
						b.modules = sliceModules(b.modules, upTo, idOrders)
						return b
					})
				}
				return m
			})
	}
	function extractFlow(previewMode: 'upTo' | 'whole'): Flow {
		if (previewMode === 'whole') {
			return $flowStore
		} else {
			const flow: Flow = JSON.parse(JSON.stringify($flowStore))
			const idOrders = dfs(flow.value.modules)
			let upToIndex = idOrders.indexOf($selectedId)

			if (upToIndex != -1) {
				flow.value.modules = sliceModules(flow.value.modules, upToIndex, idOrders)
			}
			return flow
		}
	}

	export async function runPreview(args: Record<string, any>) {
		jobProgressReset()
		const newFlow = extractFlow(previewMode)
		jobId = await runFlowPreview(args, newFlow)
		isRunning = true
	}

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Enter':
					if (event.ctrlKey) {
						event.preventDefault()
						runPreview($previewArgs)
					}
					break
			}
		}
	}

	function onJobsLoaded(newJob: Job | undefined) {
		job = newJob
		if (job?.type === 'CompletedJob') {
			isRunning = false
		}
	}
</script>

<CapturePayload bind:this={capturePayload} />

<svelte:window on:keydown={onKeyDown} />

<div class="flex divide-y flex-col space-y-2 h-screen bg-white px-6 py-2 w-full">
	<div class="flex flex-row justify-between w-full items-center gap-x-2">
		<Button
			variant="border"
			size="lg"
			color="dark"
			btnClasses="!p-0 !w-16 !h-full"
			on:click={() => {
				dispatch('close')
			}}
		>
			<Icon data={faClose} />
		</Button>

		{#if isRunning}
			<Button
				disabled={!isValid}
				color="red"
				on:click={async () => {
					isRunning = false
					try {
						jobId &&
							(await JobService.cancelQueuedJob({
								workspace: $workspaceStore ?? '',
								id: jobId,
								requestBody: {}
							}))
					} catch {}
					jobId = undefined
				}}
				size="md"
				btnClasses="w-full"
			>
				Cancel
			</Button>
		{:else}
			<Button
				variant="contained"
				startIcon={{ icon: isRunning ? faRefresh : faPlay }}
				color="blue"
				size="sm"
				btnClasses="w-full"
				disabled={!isValid}
				on:click={() => runPreview($previewArgs)}
			>
				Test flow <Kbd class="ml-2">Ctrl+Enter</Kbd>
			</Button>
		{/if}
		<Button
			btnClasses="h-full"
			size="sm"
			variant="border"
			on:click={() => {
				capturePayload.openDrawer()
			}}>Fill test args from a request</Button
		>
	</div>
	<FlowProgressBar {job} bind:reset={jobProgressReset} />

	<div class="overflow-y-auto grow flex-col flex divide-y divide-gray-600 ">
		<SchemaForm
			compact
			class="py-4"
			schema={$flowStore.schema}
			bind:isValid
			bind:args={$previewArgs}
		/>

		<div class="h-full pt-4 grow">
			{#if jobId}
				<FlowStatusViewer
					bind:flowState={$flowStateStore}
					{jobId}
					on:jobsLoaded={({ detail }) => onJobsLoaded(detail)}
				/>
			{:else}
				<div class="italic text-gray-500 h-full grow"> Flow status will be displayed here </div>
			{/if}
		</div>
	</div>
</div>
