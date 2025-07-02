<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import type { Job } from '$lib/gen'
	import { createEventDispatcher, tick } from 'svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Play } from 'lucide-svelte'
	import { writable, type Writable } from 'svelte/store'
	import type { DurationStatus, GraphModuleState } from '$lib/components/graph'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

	interface Props {
		loading?: boolean
		localModuleStates?: Writable<Record<string, GraphModuleState>>
		job?: Job
		isOwner?: boolean
		onRunPreview?: () => void
		isRunning?: boolean
		previewOpen?: boolean
	}

	let {
		loading = false,
		localModuleStates = $bindable(writable({})),
		job = $bindable(undefined),
		isOwner = $bindable(false),
		onRunPreview = $bindable(undefined),
		isRunning = $bindable(false),
		previewOpen = $bindable(false)
	}: Props = $props()

	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewMode: 'upTo' | 'whole' = $state('whole')
	let deferContent = $state(false)

	export async function openPreview(test: boolean = false) {
		if (!previewOpen) {
			previewOpen = true
			await tick()
			flowPreviewContent?.refresh()
			if (!test) return
		}
		flowPreviewContent?.test()
	}

	export async function runPreview() {
		if (!previewOpen) {
			deferContent = true
			await tick()
		}
		flowPreviewContent?.refresh
		flowPreviewContent?.test()
	}

	export function cancelTest() {
		flowPreviewContent?.cancelTest()
	}

	const dispatch = createEventDispatcher()

	let flowPreviewContent: FlowPreviewContent | undefined = $state(undefined)
	let jobId: string | undefined = $state(undefined)
	let preventEscape = $state(false)
	let selectedJobStep: string | undefined = $state(undefined)
	let selectedJobStepIsTopLevel: boolean | undefined = $state(undefined)
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = $state('single')
	let branchOrIterationN: number = $state(0)
	let scrollTop: number = $state(0)

	let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' =
		$state('timeline')

	let localDurationStatuses: Writable<Record<string, DurationStatus>> = $state(writable({}))

	let upToDisabled = $derived(
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
			$selectedId?.includes('branch') ||
			aiChatManager.flowAiChatHelpers?.getModuleAction($selectedId) === 'removed'
	)

	export function testUpTo() {
		if (upToDisabled) return
		previewMode = 'upTo'
		//previewOpen = false
		flowPreviewContent?.refresh()
		flowPreviewContent?.test()
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
	<Drawer bind:open={previewOpen} size="75%" {preventEscape} alwaysOpen={deferContent}>
		<FlowPreviewContent
			bind:this={flowPreviewContent}
			bind:localModuleStates
			bind:localDurationStatuses
			open={previewOpen}
			bind:scrollTop
			bind:previewMode
			bind:job
			bind:jobId
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
			bind:isOwner
			{onRunPreview}
			bind:isRunning
			render={previewOpen || deferContent}
		/>
	</Drawer>
{/if}
