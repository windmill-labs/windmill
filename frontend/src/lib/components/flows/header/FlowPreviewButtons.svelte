<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import type { Job } from '$lib/gen'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import { Play } from 'lucide-svelte'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

	export async function openPreview() {
		if (!previewOpen) {
			previewOpen = true
		} else {
			flowPreviewContent?.test()
		}
	}

	let flowPreviewContent: FlowPreviewContent
	let jobId: string | undefined = undefined
	let job: Job | undefined = undefined

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
			'inputs',
			'schedules',
			'failure',
			'constants',
			'Result',
			'Input'
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
	}}
	startIcon={{ icon: Play }}
	id="flow-editor-test-flow"
>
	Test flow
</Button>

<Drawer bind:open={previewOpen} alwaysOpen size="75%">
	<FlowPreviewContent
		bind:this={flowPreviewContent}
		open={previewOpen}
		bind:previewMode
		bind:job
		bind:jobId
		on:close={() => {
			previewOpen = false
		}}
	/>
</Drawer>
