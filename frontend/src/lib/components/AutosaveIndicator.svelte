<script lang="ts">
	import { untrack } from 'svelte'
	import { CloudCheck, RefreshCcw } from 'lucide-svelte'
	import type { UserDraftItemKind } from '$lib/gen'
	import { UserDraftDbSyncer, type UserDraftSyncState } from '$lib/userDraftDbSyncer.svelte'

	let {
		workspace,
		itemKind,
		path
	}: { workspace: string; itemKind: UserDraftItemKind; path: string } = $props()

	// Recompute the handle when the target draft changes; its `.state` getter
	// is itself reactive to the autosave pipeline, so `syncState` tracks both.
	const handle = $derived(UserDraftDbSyncer.getState({ workspace, itemKind, path }))
	const syncState: UserDraftSyncState = $derived(handle.state)

	// The "Saved" label is shown only for a few seconds after a save actually
	// completes (saving → none). Other transitions to `none` (e.g. a discarded
	// pending change) leave just the idle cloud-check icon with no label.
	const SAVED_LABEL_MS = 3000
	let savedVisible = $state(false)
	let prev: UserDraftSyncState = 'none'
	let timer: ReturnType<typeof setTimeout> | undefined

	$effect(() => {
		const s = syncState
		untrack(() => {
			if (s === 'saving' || s === 'pending') {
				// Active work — drop any lingering "Saved" label.
				if (timer) {
					clearTimeout(timer)
					timer = undefined
				}
				savedVisible = false
			} else if (s === 'none' && prev === 'saving') {
				// A save just landed: flash "Saved" for SAVED_LABEL_MS.
				savedVisible = true
				if (timer) clearTimeout(timer)
				timer = setTimeout(() => {
					savedVisible = false
					timer = undefined
				}, SAVED_LABEL_MS)
			}
			prev = s
		})
	})

	$effect(() => {
		return () => {
			if (timer) clearTimeout(timer)
		}
	})

	const label = $derived(
		syncState === 'saving' || syncState === 'pending' ? 'Saving...' : savedVisible ? 'Saved' : ''
	)
</script>

<div class="flex items-center gap-1.5 text-primary min-w-[4.2rem]">
	{#if syncState === 'saving' || syncState === 'pending'}
		<RefreshCcw size={14} class="animate-spin" />
	{:else}
		<CloudCheck size={16} />
	{/if}
	{#if label}
		<span class="text-secondary text-2xs">{label}</span>
	{/if}
</div>
