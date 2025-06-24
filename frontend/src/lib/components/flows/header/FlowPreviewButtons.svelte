<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import type { Job } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Play } from 'lucide-svelte'
	import { writable, type Writable } from 'svelte/store'
	import type { DurationStatus, GraphModuleState } from '$lib/components/graph'

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

	let localModuleStates: Writable<Record<string, GraphModuleState>> = writable({})
	let localDurationStatuses: Writable<Record<string, DurationStatus>> = writable({})

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

	export function testUpTo() {
		if (upToDisabled) return
		previewMode = 'upTo'
		previewOpen = true
	}
</script>

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
	dropdownItems={!upToDisabled
		? [
				{
					label: 'Test up to ' + $selectedId,
					onClick: testUpTo
				}
			]
		: undefined}
>
	Test flow
</Button>

{#if !loading}
	<Drawer bind:open={previewOpen} size="75%" {preventEscape}>
		<FlowPreviewContent
			bind:this={flowPreviewContent}
			bind:localModuleStates
			bind:localDurationStatuses
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
