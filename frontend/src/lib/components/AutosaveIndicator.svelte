<script lang="ts">
	import { untrack } from 'svelte'
	import { CloudCheck, CloudOff, RefreshCcw, RotateCcw, Users } from 'lucide-svelte'
	import type { UserDraftItemKind } from '$lib/gen'
	import { UserDraftDbSyncer, type UserDraftSyncState } from '$lib/userDraftDbSyncer.svelte'
	import { UserDraft } from '$lib/userDraft.svelte'
	import { runResetToDeployed } from '$lib/userDraftToast'
	import Popover from './meltComponents/Popover.svelte'
	import Button from './common/button/Button.svelte'
	import Toggle from './Toggle.svelte'

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
		onResetToDeployed,
		// Set true on the first overlay response that came back with
		// `is_draft: true` — triggers the on-mount "Loaded from draft"
		// hint label and green-flash animation. Replaces the old
		// `notifyDraftLoaded` toast. We snapshot the prop at mount so
		// re-renders from prop churn don't re-trigger the hint.
		loadedFromDraft = false,
		// Number of OTHER workspace users with a draft at this path
		// (i.e. the deployed-overlay's `other_drafts_users` length).
		// When > 0, the on-mount hint takes priority over "Loaded from
		// draft" and the popover offers a "See others' drafts" button
		// that flips the OtherUsersDraftsModal open through
		// `onOpenOthersDrafts`.
		othersDraftsCount = 0,
		// Wired by the route to the bindable `othersModalOpen` it
		// threads into DraftEditorModals.
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

	// `UserDraft.has` reads `entry.state.val` (a $state), so the $derived
	// re-runs when the in-memory draft appears / disappears — flips on
	// after the first edit and back off after a successful reset.
	const hasDraft = $derived(UserDraft.has(itemKind, path, { workspace }))

	// Recompute the handle when the target draft changes; its `.state` getter
	// is itself reactive to the autosave pipeline, so `syncState` tracks both.
	const handle = $derived(UserDraftDbSyncer.getState({ workspace, itemKind, path }))
	const syncState: UserDraftSyncState = $derived(handle.state)
	const failureMessage = $derived(handle.failureMessage)

	// The "Saved" label is shown only for a few seconds after a save actually
	// completes (saving → none). A transition straight to `failed` skips the
	// "Saved" flash entirely — the persistent "Save failed" label takes its
	// slot until the next attempt clears or replaces it.
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

	// One-shot tinted → transparent backdrop behind the whole indicator.
	// Shared by the on-mount load hints below and the Ctrl/Cmd+S
	// confirmation — re-triggering replays the CSS animation (the keyed
	// span remounts). `flashActive` keeps the span mounted just long
	// enough for the animation to finish, then unmounts it so the DOM
	// stays clean. Color signals the meaning: green = "your save landed"
	// (Ctrl/Cmd+S), blue = informational load hints ("Loaded from draft",
	// "Others are working on this ...").
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

	// Explicit Ctrl/Cmd+S confirmation. `flushCount` bumps on every
	// `UserDraftDbSyncer.flush()` completion — including the no-op path
	// where the autosave had already landed everything. When the pipeline
	// is idle and not failed at that moment, flash "Saved" + the green
	// backdrop so the shortcut always gives visible feedback (a real
	// flush also flashes the label via the saving → none transition
	// above; setting `savedVisible` twice is harmless). Seed from the
	// handle's current count so a remount doesn't replay an old flush.
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
	// Stays for HINT_LABEL_MS, paired with the shared green flash above.
	// Snapshot the props at mount so we don't re-fire the hint as the
	// route re-renders the indicator.
	const HINT_LABEL_MS = 7000
	let hintLabel = $state('')
	let hintTimer: ReturnType<typeof setTimeout> | undefined

	const kindLabel = $derived(
		itemKind === 'flow' ? 'flow' : itemKind === 'app' || itemKind === 'raw_app' ? 'app' : 'script'
	)

	// Triggers when either prop becomes truthy. Each truthy transition fires
	// a fresh flash — wins precedence is computed here too (others > loaded).
	// Seed to `false` (NOT to the props' current values) so a prop that is
	// already truthy at mount counts as the first false → true transition
	// and fires the hint; routes pass `loadedFromDraft = true` immediately
	// after the overlay response comes back, so this is the common case.
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

	// "Saving..." / "Saved" / "Save failed" beat any hint; otherwise the hint
	// shows. Empty string collapses the label `<span>`. `Save failed` is
	// sticky — stays until either a fresh save attempt fires (→ "Saving...")
	// or the next attempt succeeds (→ "Saved" flash).
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

	const showResetAction = $derived(!draftOnly && hasDraft && !!onResetToDeployed)

	// "Enable auto-save" preference — browser-wide, persisted by the
	// syncer. When off, keystrokes never POST (they park for Ctrl/Cmd+S)
	// and the indicator shows a muted cloud-off so the idle check-mark
	// can't be mistaken for "everything saved".
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
</script>

<div
	class="autosave-indicator-wrap relative flex items-center gap-1.5 text-primary min-w-[5.2rem] rounded-md"
	aria-label="Autosave status"
>
	{#if flashActive}
		<!-- One-shot tinted flash behind the indicator — load hints (blue)
		     and Ctrl/Cmd+S confirmations (green) both route through
		     `triggerFlash`. Keyed on `flashKey` so re-triggering replays
		     the animation (Svelte tears the keyed block down and
		     remounts). -->
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
			<div class="relative rounded-md p-1.5 hover:bg-surface-hover cursor-pointer">
				{#if syncState === 'saving' || syncState === 'pending'}
					<RefreshCcw size={14} class="animate-spin" />
				{:else if syncState === 'failed'}
					<CloudOff size={16} class="text-red-500" />
				{:else if !autosaveEnabled}
					<!-- Muted (not red — that's the failure state): autosave is
					     deliberately off, edits only persist on Ctrl/Cmd+S. -->
					<CloudOff size={16} class="text-tertiary" />
				{:else}
					<CloudCheck size={16} />
				{/if}
			</div>
		{/snippet}

		{#snippet content()}
			<div class="flex flex-col gap-3 text-sm w-72 p-3">
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
						All changes are saved as a draft on the server. The draft is per-user — your teammates'
						editors keep their own.
					</p>
				{:else}
					<p class="text-primary text-xs">
						Auto-save is off — changes only persist when you press Ctrl/Cmd+S. The draft is per-user
						— your teammates' editors keep their own.
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
	/* Tinted fade behind the whole indicator — green for save
	   confirmations, blue for informational load hints. The animation
	   fades a per-variant custom property to transparent; `forwards`
	   keeps the end state so the backdrop disappears cleanly when the
	   keyed wrapper unmounts. */
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
