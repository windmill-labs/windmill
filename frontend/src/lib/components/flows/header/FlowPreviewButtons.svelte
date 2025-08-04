<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { createEventDispatcher, tick } from 'svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Play } from 'lucide-svelte'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'

	interface Props {
		loading?: boolean
		onRunPreview?: () => void
		onJobDone?: () => void
	}

	let { loading = false, onRunPreview, onJobDone }: Props = $props()

	const { selectedId, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

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

	export async function runPreview() {
		if (!previewOpen) {
			deferContent = true
			await tick()
		}
		previewMode = 'whole'
		flowPreviewContent?.refresh()
		flowPreviewContent?.test()
	}

	export function cancelTest() {
		flowPreviewContent?.cancelTest()
	}

	const dispatch = createEventDispatcher()

	let rightColumnSelect: 'timeline' | 'node_status' | 'node_definition' | 'user_states' =
		$state('timeline')

	let upToDisabled = $derived.by(() => {
		const upToSelected = upToId ?? $selectedId
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
				'triggers'
			].includes(upToSelected) ||
			upToSelected?.includes('branch') ||
			aiChatManager.flowAiChatHelpers?.getModuleAction(upToSelected) === 'removed'
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
					onClick: () => testUpTo($selectedId, true)
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
			on:close={() => {
				// keep the data in the preview content
				deferContent = true
				previewOpen = false
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
			dynSelectCode={flowStore.val.dyn_select_code}
			dynSelectLang={flowStore.val.dyn_select_lang}
		/>
	</Drawer>
{/if}
