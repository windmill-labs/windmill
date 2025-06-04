<script lang="ts">
	import { type Job, JobService, type FlowModule, type RestartedFrom } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import { CornerDownLeft, Play, RefreshCw, X } from 'lucide-svelte'
	import type { Schema } from '$lib/common'

	export let open: boolean

	export let jobId: string | undefined = undefined
	export let job: Job | undefined = undefined
	export let modules: FlowModule[]
	export let previewArgs: Record<string, any> = {}
	export let whileLoop = false

	export const forloopSchema: Schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema' as string | undefined,
		properties: {
			iter: {
				type: 'object',
				properties: {
					index: {
						type: 'number'
					},
					value: {
						type: 'object'
					}
				}
			}
		},
		required: [],
		type: 'object'
	}

	export const whileLoopSchema: Schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema' as string | undefined,
		properties: {
			iter: {
				type: 'object',
				properties: {
					index: {
						type: 'number'
					}
				}
			}
		},
		required: [],
		type: 'object'
	}

	let selectedJobStep: string | undefined = undefined

	let isRunning: boolean = false
	let jobProgressReset: () => void

	export function test() {
		runPreview(previewArgs, undefined)
	}

	const { flowStateStore, pathStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	export async function runPreview(
		args: Record<string, any>,
		restartedFrom: RestartedFrom | undefined
	) {
		jobProgressReset()
		const newFlow = { value: { modules }, summary: '' }
		jobId = await runFlowPreview(args, newFlow, $pathStore, restartedFrom)
		isRunning = true
	}

	function onKeyDown(event: KeyboardEvent) {
		if (open) {
			switch (event.key) {
				case 'Enter':
					if (event.ctrlKey || event.metaKey) {
						event.preventDefault()
						runPreview(previewArgs, undefined)
					}
					break
			}
		}
	}

	$: if (job?.type === 'CompletedJob') {
		isRunning = false
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<div class="flex flex-col space-y-2 h-screen bg-surface px-6 py-2 w-full" id="flow-preview-content">
	<div class="flex flex-row justify-between w-full items-center gap-x-2">
		<div class="w-8">
			<Button
				on:click={() => dispatch('close')}
				startIcon={{ icon: X }}
				iconOnly
				size="sm"
				color="light"
				btnClasses="hover:bg-surface-hover  bg-surface-secondaryw-8 h-8 rounded-full p-0"
			/>
		</div>

		{#if isRunning}
			<Button
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
				}}
				size="sm"
				btnClasses="w-full max-w-lg"
				loading={true}
				clickableWhileLoading
			>
				Cancel
			</Button>
		{:else}
			<Button
				variant="contained"
				startIcon={{ icon: isRunning ? RefreshCw : Play }}
				color="dark"
				size="sm"
				btnClasses="w-full max-w-lg"
				on:click={() => runPreview(previewArgs, undefined)}
				id="flow-editor-test-flow-drawer"
				shortCut={{
					Icon: CornerDownLeft
				}}
			>
				Test iteration
			</Button>
		{/if}
		<div></div>
	</div>
	<div class="w-full flex flex-col gap-y-1">
		<FlowProgressBar {job} bind:reset={jobProgressReset} />
	</div>
	<div class="overflow-y-auto grow pr-4">
		<div class="max-h-1/2 overflow-auto border-b">
			<SchemaForm
				noVariablePicker
				compact
				className="py-4 max-w-3xl"
				schema={whileLoop ? whileLoopSchema : forloopSchema}
				bind:args={previewArgs}
			/>
		</div>
		<div class="pt-4 grow">
			{#if jobId}
				<FlowStatusViewer
					{flowStateStore}
					{jobId}
					on:jobsLoaded={({ detail }) => {
						job = detail
					}}
					bind:selectedJobStep
				/>
			{:else}
				<div class="italic text-tertiary h-full grow"> Flow status will be displayed here </div>
			{/if}
		</div>
	</div>
</div>
