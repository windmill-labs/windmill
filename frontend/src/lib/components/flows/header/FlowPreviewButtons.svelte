<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

	$: upToDisabled = [
		'settings',
		'settings-schedule',
		'settings-retries',
		'inputs',
		'schedules',
		'failure'
	].includes($selectedId)
</script>

<div class="flex flex-row-reverse justify-between items-center gap-x-2">
	<Button
		on:click={() => {
			previewMode = 'whole'
			previewOpen = !previewOpen
		}}
		size="sm"
		endIcon={{ icon: faPlay }}
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
			endIcon={{ icon: faPlay }}
		>
			Test up to step {$selectedId
				.split('-')
				.map((x) => (Number.isNaN(Number(x)) ? x : Number(x) + 1))
				.join(' ')}
		</Button>
	{/if}
</div>

<Drawer bind:open={previewOpen} size="800px">
	<FlowPreviewContent
		open={previewOpen}
		bind:previewMode
		on:close={() => {
			previewOpen = false
		}}
	/>
</Drawer>
