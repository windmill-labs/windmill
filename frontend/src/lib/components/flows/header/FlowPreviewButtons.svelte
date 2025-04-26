<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import type { Job } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Play } from 'lucide-svelte'

	export let loading = false

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

	export async function openPreview(test: boolean = false) {
		if (!previewOpen) {
			previewOpen = true
			flowPreviewContent?.refresh()
			if (!test) return
		}
		flowPreviewContent?.test()
	}

	const dispatch = createEventDispatcher()

	let flowPreviewContent: FlowPreviewContent
	let jobId: string | undefined = undefined
	let job: Job | undefined = undefined
	let preventEscape = false
	let initial = false
	let selectedJobStep: string | undefined = undefined
	let selectedJobStepIsTopLevel: boolean | undefined = undefined
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = 'single'
	let branchOrIterationN: number = 0
	let scrollTop: number = 0

	let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' = 'timeline'

	$: upToDisabled =
		$selectedId == undefined ||
		[
			'settings',
			'settings-metadata',
			'settings-schedule',
			'settings-retries',
			'settings-same-worker',
			'settings-graph',
			'settings-worker-group',
			'settings-cache',
			'settings-concurrency',
			'settings-early-stop',
			'settings-early-return',
			'inputs',
			'schedules',
			'failure',
			'preprocessor',
			'constants',
			'Result',
			'Input',
			'triggers'
		].includes($selectedId) ||
		$selectedId?.includes('branch')
</script>

{#if !upToDisabled}
	<Button
		size="xs"
		disabled={upToDisabled}
		color="light"
		variant="border"
		on:click={() => {
			previewMode = 'upTo'
			previewOpen = !previewOpen
		}}
		startIcon={{ icon: Play }}
	>
		Test up to&nbsp;
		<Badge baseClass="ml-1" small color="indigo" wrapperClass="max-h-[15px]">
			{$selectedId}
		</Badge>
	</Button>
{/if}

<Button
	color="dark"
	size="xs"
	on:click={() => {
		previewMode = 'whole'
		previewOpen = !previewOpen
		if (previewOpen) {
			flowPreviewContent?.refresh()
		}
	}}
	startIcon={{ icon: Play }}
	id="flow-editor-test-flow"
>
	Test flow
</Button>

{#if !loading}
	<Drawer bind:open={previewOpen} alwaysOpen size="75%" {preventEscape}>
		<FlowPreviewContent
			bind:this={flowPreviewContent}
			open={previewOpen}
			bind:scrollTop
			bind:previewMode
			bind:job
			bind:jobId
			bind:initial
			bind:selectedJobStep
			bind:selectedJobStepIsTopLevel
			bind:selectedJobStepType
			bind:branchOrIterationN
			bind:rightColumnSelect
			on:close={() => {
				previewOpen = false
			}}
			on:openTriggers={(e) => {
				previewOpen = false
				dispatch('openTriggers', e.detail)
			}}
			bind:preventEscape
		/>
	</Drawer>
{/if}
