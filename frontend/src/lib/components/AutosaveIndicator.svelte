<script lang="ts">
	import { untrack } from 'svelte'
	import { CloudCheck, CloudOff, RefreshCcw, RotateCcw, Users, Eye } from 'lucide-svelte'
	import type { UserDraftItemKind } from '$lib/gen'
	import { UserDraftDbSyncer, type UserDraftSyncState } from '$lib/userDraftDbSyncer.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { OtherUserDraftLoad } from '$lib/components/otherUserDraftLoad.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import Toggle from './Toggle.svelte'

	let {
		workspace,
		itemKind,
		path,
		// True when on a per-user draft with no deployed row: popover hides
		// "Reset to deployed" since there's nothing to fall back to.
		draftOnly = false,
		// Stops sync, POSTs `value: null`, restarts sync after two ticks.
		onResetToDeployed,
		// Set on the first `is_draft: true` overlay response; fires the
		// on-mount "Loaded from draft" hint. Snapshotted at mount.
		loadedFromDraft = false,
		// Count of OTHER users with a draft here. >0 makes the on-mount hint
		// take priority over "Loaded from draft" and offers "See others' drafts".
		othersDraftsCount = 0,
		onOpenOthersDrafts
	}: {
		workspace: string
		itemKind: UserDraftItemKind
		path: string
		draftOnly?: boolean
		onResetToDeployed?: () => void | Promise<void>
		loadedFromDraft?: boolean
		othersDraftsCount?: number
		onOpenOthersDrafts?: () => void
	} = $props()

	// Reactive to the in-memory draft appearing/disappearing.
	const hasDraft = $derived(UserDraft.has(itemKind, path, { workspace }))

	const handle = $derived(UserDraftDbSyncer.getState({ workspace, itemKind, path }))
	const syncState: UserDraftSyncState = $derived(handle.state)
	const failureMessage = $derived(handle.failureMessage)

	// "Saved" flashes for a few seconds on saving → none. A jump straight to
	// `failed` skips it; the sticky "Save failed" label takes the slot instead.
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
			} else if (s === 'failed') {
				// Failure replaces any in-flight "Saved" promise.
				if (timer) {
					clearTimeout(timer)
					timer = undefined
				}
				savedVisible = false
			}
			prev = s
		})
	})

	$effect(() => {
		return () => {
			if (timer) clearTimeout(timer)
		}
	})

	// One-shot tinted backdrop. Bumping `flashKey` remounts the keyed span to
	// replay the CSS animation; `flashActive` unmounts it once done. Color:
	// green = save landed (Ctrl/Cmd+S), blue = load hints.
	const FLASH_MS = 2000
	let flashKey = $state(0)
	let flashActive = $state(false)
	let flashColor = $state<'green' | 'blue'>('green')
	let flashTimer: ReturnType<typeof setTimeout> | undefined

	function triggerFlash(color: 'green' | 'blue') {
		flashKey++
		flashColor = color
		flashActive = true
		if (flashTimer) clearTimeout(flashTimer)
		flashTimer = setTimeout(() => {
			flashActive = false
			flashTimer = undefined
		}, FLASH_MS)
	}

	$effect(() => {
		return () => {
			if (flashTimer) clearTimeout(flashTimer)
		}
	})

	// Ctrl/Cmd+S feedback. `flushCount` bumps on every flush() completion,
	// including no-ops. When idle and not failed, flash "Saved" + green so the
	// shortcut always confirms. Seeded from the count so a remount doesn't replay.
	let prevFlushCount: number | undefined = undefined
	$effect(() => {
		const count = handle.flushCount
		const s = syncState
		untrack(() => {
			const isFirstRead = prevFlushCount === undefined
			const bumped = !isFirstRead && count > (prevFlushCount ?? 0)
			prevFlushCount = count
			if (!bumped) return
			if (s === 'saving' || s === 'pending' || s === 'failed') return
			savedVisible = true
			triggerFlash('green')
			if (timer) clearTimeout(timer)
			timer = setTimeout(() => {
				savedVisible = false
				timer = undefined
			}, SAVED_LABEL_MS)
		})
	})

	// On-mount load hint — "Others are working..." or "Loaded from draft".
	const HINT_LABEL_MS = 7000
	let hintLabel = $state('')
	let hintTimer: ReturnType<typeof setTimeout> | undefined

	const kindLabel = $derived(
		itemKind === 'flow' ? 'flow' : itemKind === 'app' || itemKind === 'raw_app' ? 'app' : 'script'
	)

	// Each false → true transition fires a hint (others > loaded). Seeded to
	// `false`, NOT the props' values, so a prop already truthy at mount counts
	// as a transition and fires — the common case once the overlay responds.
	let prevOthers = false
	let prevLoaded = false
	$effect(() => {
		const othersNow = othersDraftsCount > 0
		const loadedNow = !!loadedFromDraft
		untrack(() => {
			const othersTransition = othersNow && !prevOthers
			const loadedTransition = loadedNow && !prevLoaded
			if (othersTransition || loadedTransition) {
				hintLabel = othersNow ? `Others are working on this ${kindLabel}` : 'Loaded from draft'
				triggerFlash('blue')
				if (hintTimer) clearTimeout(hintTimer)
				hintTimer = setTimeout(() => {
					hintLabel = ''
					hintTimer = undefined
				}, HINT_LABEL_MS)
			}
			prevOthers = othersNow
			prevLoaded = loadedNow
		})
	})

	$effect(() => {
		return () => {
			if (hintTimer) clearTimeout(hintTimer)
		}
	})

	// Status labels beat any hint; "" collapses the label span. "Save failed"
	// is sticky until the next attempt fires or succeeds.
	const label = $derived(
		syncState === 'saving' || syncState === 'pending'
			? 'Saving...'
			: syncState === 'failed'
				? 'Save failed'
				: savedVisible
					? 'Saved'
					: hintLabel
	)
	const labelIsError = $derived(syncState === 'failed')

	// Overlay mode: editing another user's loaded draft. Autosave is hard-locked,
	// so the toggle is meaningless; offer "Reset to draft" (restore our own) instead.
	const editingOtherUserDraft = $derived(OtherUserDraftLoad.isActive(workspace, itemKind, path))
	const otherDraftOwnerLabel = $derived(
		OtherUserDraftLoad.getSession(workspace, itemKind, path)?.ownerLabel
	)

	const showResetAction = $derived(!draftOnly && hasDraft && !!onResetToDeployed)

	// "Enable auto-save" preference — browser-wide, persisted by the syncer.
	// When off, keystrokes park for Ctrl/Cmd+S and the indicator shows a muted
	// cloud-off so the idle check-mark can't read as "everything saved".
	const autosaveEnabled = $derived(UserDraftDbSyncer.autosaveEnabled)

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

	function openOthersDrafts() {
		popoverOpen = false
		onOpenOthersDrafts?.()
	}

	let resettingToOwnDraft = $state(false)
	async function resetToOwnDraft() {
		if (resettingToOwnDraft) return
		resettingToOwnDraft = true
		try {
			await OtherUserDraftLoad.resetToOwnDraft(workspace, itemKind, path)
		} finally {
			resettingToOwnDraft = false
			popoverOpen = false
		}
	}
</script>

<div
	class="autosave-indicator-wrap relative flex items-center gap-1.5 text-primary shrink-0 rounded-md"
	aria-label="Autosave status"
>
	{#if flashActive}
		<!-- Keyed on `flashKey` so re-triggering remounts and replays the animation. -->
		{#key flashKey}
			<span
				class="autosave-hint-flash absolute inset-0 rounded-md pointer-events-none"
				style="--autosave-flash-color: {flashColor === 'green'
					? 'rgba(34, 197, 94, 0.22)'
					: 'rgba(59, 130, 246, 0.22)'}"
				aria-hidden="true"
			></span>
		{/key}
	{/if}
	<Popover
		bind:isOpen={popoverOpen}
		placement="bottom-end"
		usePointerDownOutside
		closeOnOutsideClick
	>
		{#snippet trigger()}
			<div
				class="relative rounded-md p-1.5 hover:bg-surface-hover cursor-pointer"
				title={syncState === 'failed'
					? `Save failed${failureMessage ? `: ${failureMessage}` : ''} — click for details`
					: undefined}
			>
				{#if editingOtherUserDraft}
					<!-- Viewing another user's draft: not saved, distinct from the saved check-mark. -->
					<Eye size={16} class="text-blue-500" />
				{:else if syncState === 'saving' || syncState === 'pending'}
					<RefreshCcw size={14} class="animate-spin" />
				{:else if syncState === 'failed'}
					<CloudOff size={16} class="text-red-500" />
				{:else if !autosaveEnabled}
					<!-- Muted, not red (the failure state): autosave is off, edits persist on Ctrl/Cmd+S. -->
					<CloudOff size={16} class="text-tertiary" />
				{:else}
					<CloudCheck size={16} />
				{/if}
			</div>
		{/snippet}

		{#snippet content()}
			<div class="flex flex-col gap-3 text-sm w-72 p-3">
				{#if editingOtherUserDraft}
					<p class="text-primary text-xs">
						You're editing {otherDraftOwnerLabel ?? 'another user'}'s draft. Auto-save is paused —
						your own draft is untouched. Editing prompts before overwriting it.
					</p>
					<Button
						variant="default"
						size="xs"
						loading={resettingToOwnDraft}
						startIcon={{ icon: RotateCcw }}
						on:click={() => void resetToOwnDraft()}
					>
						Reset to draft
					</Button>
				{:else}
					{#if syncState === 'failed'}
						<div class="flex flex-col gap-1">
							<p class="text-red-500 font-semibold text-xs">Save failed</p>
							{#if failureMessage}
								<pre
									class="text-red-500 text-xs whitespace-pre-wrap break-words font-mono max-h-40 overflow-y-auto"
									>{failureMessage}</pre
								>
							{/if}
						</div>
					{/if}
					{#if autosaveEnabled}
						<p class="text-primary text-xs">
							All changes are saved as a draft on the server. The draft is per-user — your
							teammates' editors keep their own.
						</p>
					{:else}
						<p class="text-primary text-xs">
							Auto-save is off — changes only persist when you press Ctrl/Cmd+S. The draft is
							per-user — your teammates' editors keep their own.
						</p>
					{/if}
					<Toggle
						size="xs"
						checked={autosaveEnabled}
						options={{ right: 'Enable auto-save' }}
						on:change={(e) => {
							UserDraftDbSyncer.autosaveEnabled = e.detail
						}}
					/>
					{#if othersDraftsCount > 0}
						<div class="flex flex-col gap-2 border-t pt-3">
							<p class="text-primary text-xs">
								Other users are working on this {kindLabel}.
							</p>
							<Button
								variant="default"
								size="xs"
								startIcon={{ icon: Users }}
								on:click={openOthersDrafts}
							>
								See others' drafts
							</Button>
						</div>
					{/if}
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
				{/if}
			</div>
		{/snippet}
	</Popover>
	{#if label}
		<span
			class="relative text-2xs pr-1.5 {labelIsError
				? 'text-red-500 font-semibold'
				: 'text-secondary'}">{label}</span
		>
	{/if}
</div>

<style>
	/* Fades the per-variant custom property to transparent; `forwards` keeps
	   the end state so the backdrop disappears cleanly on unmount. */
	@keyframes autosave-hint-flash-anim {
		0% {
			background-color: var(--autosave-flash-color);
		}
		100% {
			background-color: transparent;
		}
	}
	.autosave-hint-flash {
		animation: autosave-hint-flash-anim 1.8s ease-out forwards;
	}
</style>
