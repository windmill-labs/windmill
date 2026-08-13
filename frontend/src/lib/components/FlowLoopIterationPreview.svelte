<script lang="ts">
	import { type Job, JobService, type FlowModule, type RestartedFrom } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import { runFlowPreview } from './flows/utils.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import FlowStatusViewer from '../components/FlowStatusViewer.svelte'
	import FlowProgressBar from './flows/FlowProgressBar.svelte'
	import { CornerDownLeft, Play, RefreshCw, X } from 'lucide-svelte'
	import type { Schema } from '$lib/common'

	interface Props {
		open: boolean
		jobId?: string | undefined
		job?: Job | undefined
		modules: FlowModule[]
		previewArgs?: Record<string, any>
		whileLoop?: boolean
	}

	let {
		open,
		jobId = $bindable(undefined),
		job = $bindable(undefined),
		modules,
		previewArgs = $bindable({}),
		whileLoop = false
	}: Props = $props()

	export const forloopSchema: Schema = {
		$schema: 'https://json-schema.org/draft/2020-12/schema' as string | undefined,
		properties: {
			iter: {
				type: 'object',
				description: 'The loop iterator, exposed to the steps below as flow_input.iter',
				properties: {
					index: {
						type: 'integer',
						min: 0,
						description: "Position in the iterator's sequence. The first iteration is 0."
					},
					value: {
						type: 'object',
						description: "The element of the iterator's sequence this iteration receives."
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
				description: 'The loop iterator, exposed to the steps below as flow_input.iter',
				properties: {
					index: {
						type: 'integer',
						min: 0,
						description:
							'How many iterations have already run. The first iteration is 0. A while loop has no sequence, so it sets iter.value to this same number.'
					}
				}
			}
		},
		required: [],
		type: 'object'
	}

	// A real while loop always sets iter.value to iter.index, and counts whole iterations from
	// 0. The preview flow holds only the loop body, so nothing else supplies that context:
	// mirror the value and clamp to a whole, non-negative index so a preview matches a run.
	function withWhileLoopIter(args: Record<string, any>): Record<string, any> {
		const index = Math.max(0, Math.floor(args.iter?.index ?? 0))
		return { ...args, iter: { ...args.iter, index, value: index } }
	}

	let selectedJobStep: string | undefined = $state(undefined)

	let isRunning: boolean = $state(false)
	let progressBar: FlowProgressBar | undefined = $state(undefined)

	export function test() {
		runPreview(previewArgs, undefined)
	}

	const { flowStateStore, flowStore, pathStore, opWorkspace } =
		getContext<FlowEditorContext>('FlowEditorContext')
	const dispatch = createEventDispatcher()

	export async function runPreview(
		args: Record<string, any>,
		restartedFrom: RestartedFrom | undefined
	) {
		progressBar?.reset()
		// The preview flow holds only the loop body, so it inherits none of the flow's settings:
		// carry the tag over so the iteration lands on the worker group the flow runs on.
		const newFlow = { value: { modules }, summary: '', tag: flowStore.val.tag }
		jobId = await runFlowPreview(
			whileLoop ? withWhileLoopIter(args) : args,
			newFlow,
			$pathStore,
			restartedFrom,
			undefined,
			undefined,
			opWorkspace?.()
		)
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

	$effect(() => {
		if (job?.type === 'CompletedJob') {
			isRunning = false
		}
	})
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="flex flex-col space-y-2 h-screen bg-surface px-6 py-2 w-full" id="flow-preview-content">
	<div class="flex flex-row justify-between w-full items-center gap-x-2">
		<div class="w-8">
			<Button
				on:click={() => dispatch('close')}
				startIcon={{ icon: X }}
				iconOnly
				unifiedSize="md"
				variant="default"
				btnClasses="hover:bg-surface-hover  bg-surface-secondaryw-8 h-8 rounded-full p-0"
			/>
		</div>

		{#if isRunning}
			<Button
				variant="accent"
				destructive
				on:click={async () => {
					isRunning = false
					try {
						jobId &&
							(await JobService.cancelQueuedJob({
								workspace: opWorkspace?.() ?? $workspaceStore ?? '',
								id: jobId,
								requestBody: {}
							}))
					} catch {}
				}}
				unifiedSize="md"
				btnClasses="w-full max-w-lg"
				loading={true}
				clickableWhileLoading
			>
				Cancel
			</Button>
		{:else}
			<Button
				variant="accent"
				startIcon={{ icon: isRunning ? RefreshCw : Play }}
				unifiedSize="md"
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
	{#if jobId}
		<div class="w-full flex flex-col gap-y-1">
			<FlowProgressBar {job} bind:this={progressBar} />
		</div>
	{/if}
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
					bind:flowState={flowStateStore.val}
					workspaceId={opWorkspace?.()}
					{jobId}
					onJobsLoaded={({ job: newJob }) => {
						job = newJob
					}}
					bind:selectedJobStep
				/>
			{:else}
				<div class="italic text-primary h-full grow"> Flow status will be displayed here </div>
			{/if}
		</div>
	</div>
</div>
