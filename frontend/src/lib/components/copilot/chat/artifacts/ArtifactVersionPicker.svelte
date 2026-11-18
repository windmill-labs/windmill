<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import TimeAgo from '$lib/components/TimeAgo.svelte'
	import { ChevronDown } from 'lucide-svelte'
	import { displayDate } from '$lib/utils'
	import { currentVersion, type ArtifactVersion, type PersistedArtifact } from './artifactsDB'
	import type { SessionArtifactsStore } from './artifactsState.svelte'

	interface Props {
		artifact: PersistedArtifact
		store: SessionArtifactsStore
		/** Version being shown, or undefined for the current one. */
		selected: number | undefined
		onSelect: (version: number | undefined) => void
	}

	let { artifact, store, selected, onSelect }: Props = $props()

	const latest = $derived(currentVersion(artifact))
	const shown = $derived(selected ?? latest)

	let open = $state(false)
	let versions = $state<ArtifactVersion[]>([])

	// Loaded on open, not on mount: a snapshot row carries its whole content, so listing
	// twenty of them is far more than the trigger needs to know that history exists.
	// Reloaded on every later version too, since the assistant can write one while the menu
	// is open — the rows would then contradict the count in the header.
	$effect(() => {
		// Reads `latest` only while open, which is exactly when a reload is worth doing.
		if (open && latest) void load()
	})

	// Reopening the menu while the assistant is writing puts two loads in flight. Only the
	// last one asked for may win: an earlier list landing last would be rendered under a
	// header counted against the newer `latest`, claiming history had been pruned when it
	// had not.
	let loadSeq = 0
	async function load() {
		const seq = ++loadSeq
		const loaded = await store.listVersions(artifact.id)
		if (seq === loadSeq) versions = loaded
	}

	// v1 was created; anything later was edited — a missing note must not claim otherwise.
	function noteLabel(v: ArtifactVersion): string {
		return v.note ?? (v.version === 1 ? 'Created' : 'Edited')
	}

	function pick(version: number) {
		open = false
		onSelect(version === latest ? undefined : version)
	}
</script>

<Popover
	bind:isOpen={open}
	placement="bottom-start"
	closeOnOtherPopoverOpen
	contentClasses="!bg-surface"
	triggerAttrs={{ 'aria-label': `Version ${shown} of ${latest}`, 'aria-haspopup': 'listbox' }}
	class="shrink-0"
>
	{#snippet trigger()}
		<span
			class="flex items-center gap-1 rounded px-1.5 py-0.5 text-2xs font-normal tabular-nums
				{selected !== undefined
				? 'bg-orange-100 text-orange-800 dark:bg-orange-950 dark:text-orange-300'
				: 'bg-surface-secondary text-secondary hover:text-emphasis'}"
		>
			v{shown}
			<ChevronDown size={11} />
		</span>
	{/snippet}

	{#snippet content()}
		<div class="flex w-80 flex-col text-xs">
			<!-- Version numbers run 1..latest, so a count says nothing the rows don't — until
			     pruning drops the oldest, which is the one thing the list cannot show. -->
			<div class="px-3 pt-2 pb-1 text-2xs text-hint">
				{#if versions.length > 0 && versions.length < latest}
					{versions.length} most recent of {latest} versions
				{:else}
					Versions
				{/if}
			</div>
			<div class="max-h-[min(20rem,60vh)] overflow-y-auto py-1" role="listbox" tabindex="-1">
				{#each versions as v (v.version)}
					<button
						type="button"
						role="option"
						aria-selected={v.version === shown}
						title={noteLabel(v)}
						class="flex w-full flex-col gap-0.5 px-3 py-1.5 text-left font-normal
							{v.version === shown ? 'bg-surface-accent-selected' : 'hover:bg-surface-hover'}"
						onclick={() => pick(v.version)}
					>
						<span class="line-clamp-2 w-full text-primary">
							{noteLabel(v)}
						</span>
						<span class="text-2xs tabular-nums text-hint" title={displayDate(new Date(v.savedAt))}>
							v{v.version} · <TimeAgo date={new Date(v.savedAt).toISOString()} compact /> ago
						</span>
					</button>
				{/each}
			</div>
		</div>
	{/snippet}
</Popover>
