<script lang="ts">
	import { Job, JobService, type Flow, type FlowModule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faClose, faPlay, faRefresh } from '@fortawesome/free-solid-svg-icons'
	import { Button } from './common'
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
	<div class="flex flex-row justify-between w-full items-center gap-x-1">
		<Button
			variant="border"
			size="lg"
			color="dark"
			btnClasses="!p-0 !w-8 !h-8"
			on:click={() => {
				dispatch('close')
			}}
		>
			<Icon data={faClose} />
		</Button>

		<div class="flex flex-row justify-center items-center">
			<div class="flex justify-center p-2 w-8 h-8 bg-blue-200 rounded-lg mr-2 ">
				<Icon data={faPlay} scale={1} class="text-blue-500" />
			</div>

			<h3
				class="text-lg leading-6 font-bold text-gray-900 inline-flex flex-row justify-between w-full"
			>
				<div>
					Test {previewMode === 'upTo'
						? `up to step ${$selectedId.split('-').join(',')}`
						: ' whole flow'}
				</div>
			</h3>
		</div>
		<Button
			btnClasses="ml-2"
			size="sm"
			variant="border"
			on:click={() => {
				capturePayload.openDrawer()
			}}>Fill test args from payload</Button
		>
	</div>
	<div class="h-full grow pb-20 max-h-60 overflow-auto">
		<SchemaForm
			class="h-full pt-4"
			schema={$flowStore.schema}
			bind:isValid
			bind:args={$previewArgs}
		/>
	</div>
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
		>
			Cancel
		</Button>
	{:else}
		<Button
			variant="contained"
			endIcon={{ icon: isRunning ? faRefresh : faPlay }}
			color="blue"
			btnClasses="w-full"
			disabled={!isValid}
			on:click={() => runPreview($previewArgs)}
		>
			Test flow (Ctrl/Cmd + Enter)
		</Button>
	{/if}

	<FlowProgressBar {job} bind:reset={jobProgressReset} class="py-4" />

	<div class="h-full overflow-y-auto mb-16 pt-4 grow">
		{#if jobId}
			<FlowStatusViewer
				bind:flowState={$flowStateStore}
				{jobId}
				on:jobsLoaded={({ detail }) => onJobsLoaded(detail)}
			/>
		{/if}
	</div>
</div>
