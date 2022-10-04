<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'

	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

	$: upToDisabled = [
		'settings',
		'settings-schedule',
		'settings-retries',
		'inputs',
		'schedules'
	].includes($selectedId)
</script>

<span
	class="flex flex-row-reverse justify-between flex-wrap gap-2  w-full py-2 px-4 bg-white border-b border-t"
>
	<Button
		btnClasses="grow"
		on:click={() => {
			previewMode = 'whole'
			previewOpen = !previewOpen
		}}
		size="sm"
	>
		Test flow
		<Icon data={faPlay} class="ml-2" scale={0.8} />
	</Button>
	{#if !upToDisabled}
		<Button
			btnClasses="grow"
			size="sm"
			disabled={upToDisabled}
			color="light"
			variant="border"
			on:click={() => {
				previewMode = 'upTo'

				previewOpen = !previewOpen
			}}
		>
			Test up to step {$selectedId
				.split('-')
				.map((x) => (Number.isNaN(Number(x)) ? x : Number(x) + 1))
				.join(' ')}
			<Icon data={faPlay} class="ml-2" scale={0.8} />
		</Button>
	{/if}
</span>

<Drawer bind:open={previewOpen} size="800px">
	<FlowPreviewContent bind:previewMode on:close={() => (previewOpen = !previewOpen)} />
</Drawer>
