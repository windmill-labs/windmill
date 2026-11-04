<script lang="ts">
	import { Tab, Tabs } from '$lib/components/common'
	import DiffTab from './DiffTab.svelte'
	import ReviewTab from './ReviewTab.svelte'
	import MultiFileTab from './MultiFileTab.svelte'
	import EditorTab from './EditorTab.svelte'
	import MergeTab from './MergeTab.svelte'

	let tab = $state('diff')
</script>

<svelte:head>
	<title>Diff comparison</title>
</svelte:head>

<main class="mx-auto max-w-[1440px] p-6">
	<h1 class="mb-1 text-lg font-semibold">Diff comparison</h1>
	<p class="mb-4 text-xs text-secondary">
		Monaco, our highlight.js diff view and <code>@pierre/diffs</code> on the same inputs.
	</p>
	<Tabs bind:selected={tab}>
		<Tab value="diff" label="Diff" />
		<Tab value="review" label="Review" />
		<Tab value="multi" label="Multi-file" />
		<Tab value="editor" label="Editor" />
		<Tab value="merge" label="Merge conflict" />
		{#snippet content()}
			<div class="pt-4">
				{#if tab === 'diff'}
					<DiffTab />
				{:else if tab === 'review'}
					<ReviewTab />
				{:else if tab === 'multi'}
					<MultiFileTab />
				{:else if tab === 'editor'}
					<EditorTab />
				{:else if tab === 'merge'}
					<MergeTab />
				{/if}
			</div>
		{/snippet}
	</Tabs>
</main>
