<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { createEventDispatcher, tick } from 'svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { FlowPreview } from '$lib/components/FlowPreview.svelte'
	import { Play } from 'lucide-svelte'
	import { writable, type Writable } from 'svelte/store'
	import type { DurationStatus, GraphModuleState } from '$lib/components/graph'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

	interface Props {
		loading?: boolean
		localModuleStates?: Writable<Record<string, GraphModuleState>>
		isOwner?: boolean
		previewOpen?: boolean
		flowPreview: FlowPreview
	}

	let {
		loading = false,
		localModuleStates = $bindable(writable({})),
		isOwner = $bindable(false),
		previewOpen = $bindable(false),
		flowPreview
	}: Props = $props()

	const { selectedId, previewArgs } = getContext<FlowEditorContext>('FlowEditorContext')
	let renderCount = $state(0)

	export async function openPreview(test: boolean = false) {
		if (!previewOpen) {
			previewOpen = true
			await tick()
			renderCount++
			if (!test) return
		}
		flowPreview.test(previewArgs.val)
	}

	export function runPreview() {
		renderCount++
		flowPreview.test(previewArgs.val)
	}

	export function cancelTest() {
		flowPreview.cancelTest()
	}

	const dispatch = createEventDispatcher()

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
		flowPreview.previewMode = 'upTo'
		//previewOpen = false
		renderCount++
		flowPreview.test(previewArgs.val)
	}
</script>

<Button
	color="dark"
	size="xs"
	on:click={() => {
		flowPreview.previewMode = 'whole'
		previewOpen = !previewOpen
		if (previewOpen) {
			renderCount++
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
	<Drawer bind:open={previewOpen} size="75%" {preventEscape} alwaysOpen>
		<FlowPreviewContent
			{flowPreview}
			bind:localModuleStates
			bind:localDurationStatuses
			open={previewOpen}
			bind:scrollTop
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
			refreshTrigger={renderCount}
		/>
	</Drawer>
{/if}
