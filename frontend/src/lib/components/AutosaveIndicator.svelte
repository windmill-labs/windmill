<script lang="ts">
	import { untrack } from 'svelte'
	import { CloudCheck, RefreshCcw, RotateCcw } from 'lucide-svelte'
	import type { UserDraftItemKind } from '$lib/gen'
	import { UserDraftDbSyncer, type UserDraftSyncState } from '$lib/userDraftDbSyncer.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'

	let {
		workspace,
		itemKind,
		path,
		// Reactive — when true, the indicator's popover hides "Reset to
		// deployed" because there's nothing to fall back to (the editor
		// is on a per-user draft at a path with no deployed row). Routes
		// thread their own `isNewX` / `savedX.no_deployed` here.
		draftOnly = false,
		// Route-specific reset logic. The popover button stops sync,
		// fires `value: null` at the syncer, awaits this, then restarts
		// sync after two ticks — mirrors `notifyDraftLoaded`'s "Reset to
		// deployed" toast action so the discard sticks.
		onResetToDeployed
	}: {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		draftOnly?: boolean
		onResetToDeployed?: () => void | Promise<void>
	} = $props()

	// `UserDraft.has` reads `entry.state.val` (a $state), so the $derived
	// re-runs when the in-memory draft appears / disappears — flips on
	// after the first edit and back off after a successful reset.
	const hasDraft = $derived(UserDraft.has(itemKind, path, { workspace }))

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

	const showResetAction = $derived(!draftOnly && hasDraft && !!onResetToDeployed)

	let popoverOpen = $state(false)
	let resetting = $state(false)

	async function resetToDeployed() {
		if (!onResetToDeployed || resetting) return
		resetting = true
		try {
			await runResetToDeployed({ workspace, itemKind, path, onResetToDeployed })
		} finally {
			resetting = false
			popoverOpen = false
		}
	}
</script>

<div
	class="flex items-center gap-1.5 text-primary min-w-[4.2rem]"
	aria-label="Autosave status"
>
	<Popover
		bind:isOpen={popoverOpen}
		placement="bottom-end"
		usePointerDownOutside
		closeOnOutsideClick
	>
		{#snippet trigger()}
			<div class='rounded-md p-1.5 hover:bg-surface-hover cursor-pointer'>
				{#if syncState === 'saving' || syncState === 'pending'}
					<RefreshCcw size={14} class="animate-spin" />
				{:else}
					<CloudCheck size={16}  />
				{/if}
			</div>
		{/snippet}

		{#snippet content()}
			<div class="flex flex-col gap-3 text-sm w-72 p-3">
				<p class="text-primary text-sm">
					All changes are saved as a draft on the server. The draft is per-user — your teammates'
					editors keep their own.
				</p>
				{#if showResetAction}
					<Button
						variant="default"
						size="xs"
						loading={resetting}
						startIcon={{ icon: RotateCcw }}
						on:click={() => void resetToDeployed()}
					>
						Reset to deployed
					</Button>
				{/if}
			</div>
		{/snippet} 
	</Popover>
	{#if label}
		<span class="text-secondary text-2xs">{label}</span>
	{/if}
</div>