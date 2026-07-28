<script lang="ts">
	import SimpleEditor from '../SimpleEditor.svelte'

	// Up/Down SQL editor for a data table migration. Two tabs keep the up (CREATE)
	// and down (DROP) SQL from cluttering the view; both are editable Monaco.
	let {
		up = $bindable(),
		down = $bindable(),
		// Bump to force the Monaco editors to re-mount with fresh code — Monaco does
		// not sync external `code` changes, so re-keying is how regenerated SQL shows.
		generation = 0
	}: { up: string; down: string; generation?: number } = $props()

	let tab = $state<'up' | 'down'>('up')
</script>

<div class="flex flex-col gap-1">
	<div class="flex gap-1 text-[11px]">
		{#each [{ id: 'up', label: 'Up' }, { id: 'down', label: 'Down' }] as t (t.id)}
			<button
				type="button"
				class="rounded px-2 py-0.5 font-medium {tab === t.id
					? 'bg-surface-selected text-primary'
					: 'text-secondary hover:bg-surface-hover'}"
				onclick={() => (tab = t.id as 'up' | 'down')}
			>
				{t.label}
			</button>
		{/each}
	</div>
	{#key generation}
		<div class="h-44 overflow-hidden rounded border bg-surface">
			{#if tab === 'up'}
				<SimpleEditor class="h-full" lang="sql" bind:code={up} small automaticLayout />
			{:else}
				<SimpleEditor class="h-full" lang="sql" bind:code={down} small automaticLayout />
			{/if}
		</div>
	{/key}
</div>
