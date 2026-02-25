<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { createEventDispatcher, tick } from 'svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Play } from 'lucide-svelte'
	import type { GraphModuleState } from '$lib/components/graph'
	import type { StateStore } from '$lib/utils'
	import type { Job } from '$lib/gen'

	interface Props {
		loading?: boolean
		onRunPreview?: () => void
		onJobDone?: () => void
		localModuleStates?: Record<string, GraphModuleState>
		suspendStatus: StateStore<Record<string, { job: Job; nb: number }>>
	}

	let {
		loading = false,
		onRunPreview,
		onJobDone,
		localModuleStates = $bindable({}),
		suspendStatus
	}: Props = $props()

	const { selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')

	let flowPreviewContent: FlowPreviewContent | undefined = $state(undefined)
	let preventEscape = $state(false)
	let selectedJobStep: string | undefined = $state(undefined)
	let selectedJobStepIsTopLevel: boolean | undefined = $state(undefined)
	let selectedJobStepType: 'single' | 'forloop' | 'branchall' = $state('single')
	let branchOrIterationN: number = $state(0)
	let scrollTop: number = $state(0)
	let previewMode: 'upTo' | 'whole' = $state('whole')
	let upToId: string | undefined = $state(undefined)
	let previewOpen = $state(false)
	let deferContent = $state(false)

	export async function openPreview(test: boolean = false) {
		if (!previewOpen) {
			previewOpen = true
			await tick()
			flowPreviewContent?.refresh()
			if (!test) return
		}
		previewMode = 'whole'
		flowPreviewContent?.test()
	}

	export async function openRecordingPreview() {
		if (!previewOpen) {
			previewOpen = true
			await tick()
			flowPreviewContent?.refresh()
		}
		previewMode = 'whole'
		flowPreviewContent?.setRecordingMode(true)
	}

	export async function runPreview(conversationId?: string): Promise<string | undefined> {
		if (!previewOpen) {
			deferContent = true
			await tick()
		}
		previewMode = 'whole'
		flowPreviewContent?.refresh()
		return await flowPreviewContent?.test(conversationId)
	}

	export function cancelTest() {
		flowPreviewContent?.cancelTest()
	}

	const dispatch = createEventDispatcher()

	let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' =
		$state('timeline')

	let upToDisabled = $derived.by(() => {
		const upToSelected = upToId ?? selectionManager.getSelectedId()
		return (
			upToSelected == undefined ||
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
				'Trigger'
			].includes(upToSelected) ||
			upToSelected?.includes('branch')
		)
	})

	export async function testUpTo(id: string | undefined, openPreview: boolean = false) {
		upToId = id
		if (upToDisabled) return
		if (openPreview) {
			previewOpen = true
		} else if (!previewOpen) {
			deferContent = true
			await tick()
		}
		previewMode = 'upTo'
		flowPreviewContent?.refresh()
		if (!openPreview) {
			flowPreviewContent?.test()
		}
	}

	export function getPreviewMode() {
		return previewMode
	}

	export function getPreviewOpen() {
		return previewOpen
	}

	export function getFlowPreviewContent() {
		return flowPreviewContent
	}
</script>

<Button
	variant="accent-secondary"
	wrapperClasses="whitespace-nowrap"
	unifiedSize="md"
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
					label: 'Test up to ' + selectionManager.getSelectedId(),
					onClick: () => testUpTo(selectionManager.getSelectedId(), true)
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
			open={previewOpen}
			bind:scrollTop
			bind:previewMode
			bind:selectedJobStep
			bind:selectedJobStepIsTopLevel
			bind:selectedJobStepType
			bind:branchOrIterationN
			bind:rightColumnSelect
			bind:localModuleStates
			on:close={() => {
				// keep the data in the preview content
				deferContent = true
				previewOpen = false
				flowPreviewContent?.setRecordingMode(false)
			}}
			on:openTriggers={(e) => {
				previewOpen = false
				dispatch('openTriggers', e.detail)
			}}
			bind:preventEscape
			{onRunPreview}
			render={previewOpen}
			{onJobDone}
			{upToId}
			{suspendStatus}
		/>
	</Drawer>
{/if}
