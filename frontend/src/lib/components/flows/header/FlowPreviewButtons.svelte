<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import type { Job } from '$lib/gen'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

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
			'inputs',
			'schedules',
			'failure',
			'constants'
		].includes($selectedId) ||
		$selectedId?.includes('branch')

	let is_owner = false
</script>

<div class="flex flex-row-reverse justify-between items-center gap-x-2">
	<Button
		on:click={() => {
			previewMode = 'whole'
			previewOpen = !previewOpen
		}}
		size="sm"
		startIcon={{ icon: faPlay }}
	>
		Test flow
	</Button>
	{#if !upToDisabled}
		<Button
			size="sm"
			disabled={upToDisabled}
			color="light"
			variant="border"
			on:click={() => {
				previewMode = 'upTo'
				previewOpen = !previewOpen
			}}
			startIcon={{ icon: faPlay }}
		>
			Test up to
			<Badge baseClass="ml-1" color="indigo">
				{$selectedId}
			</Badge>
		</Button>
	{/if}
</div>

<Drawer bind:open={previewOpen} alwaysOpen size="75%">
	<FlowPreviewContent
		open={previewOpen}
		bind:previewMode
		bind:job
		bind:jobId
		on:close={() => {
			previewOpen = false
		}}
	/>
</Drawer>
