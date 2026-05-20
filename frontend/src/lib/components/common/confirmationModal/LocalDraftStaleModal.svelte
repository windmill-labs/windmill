<script lang="ts">
	/**
	 * Modal shown when the four route-level editors detect that the
	 * remote state has moved on since the local autosave was captured —
	 * either a teammate (or another tab) pushed a fresh "Save as draft"
	 * via `DraftService` (`cause = 'draft'`), or the deployed version
	 * changed (`cause = 'version'`).
	 *
	 * Sits above the per-browser `UserDraft` autosave layer and is
	 * separate from the backend `DraftService` flow. The actual reset
	 * actions live at each call site; this modal just owns the
	 * "Load latest version" vs "Keep current draft" decision.
	 */
	import { classNames } from '$lib/utils'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { CornerDownLeft, RefreshCcw } from 'lucide-svelte'

	// 'url' is temporary — used by /scripts/{add,edit}'s URL-hash sync block
	// while we wait for a future PR to replace that legacy behavior.
	type Cause = 'draft' | 'version' | 'url'

	let {
		open = false,
		cause = 'version',
		onLoadLatest,
		onKeepDraft
	}: {
		open?: boolean
		/** What changed on the remote since the local draft was created. */
		cause?: Cause
		onLoadLatest: () => void | Promise<void>
		onKeepDraft: () => void | Promise<void>
	} = $props()

	function onKeyDown(event: KeyboardEvent) {
		if (!open) return
		event.stopPropagation()
		event.preventDefault()
		switch (event.key) {
			case 'Enter':
				onLoadLatest()
				break
			case 'Escape':
				onKeepDraft()
				break
		}
	}
	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}

	const title = $derived(
		cause === 'draft'
			? 'A newer draft was saved on the server'
			: cause === 'url'
				? 'The URL contains a different script payload'
				: 'A newer version was deployed on the server'
	)
	const body = $derived(
		cause === 'draft'
			? "The editor is showing your local autosave. Someone else (or another tab) pushed a newer draft to the server while you were editing — your copy is now behind. Load latest replaces what's on screen; Keep current leaves it alone."
			: cause === 'url'
				? 'The current page URL encodes a script (e.g. from a Fork link or shared URL) that differs from your local autosave. Load from URL replaces your local draft; Keep current draft ignores the URL payload.'
				: "The editor is showing your local autosave. A newer version was deployed while you were editing — your copy is now behind. Load latest replaces what's on screen; Keep current leaves it alone."
	)
	const loadLabel = $derived(cause === 'url' ? 'Load from URL' : 'Load latest version')
</script>

<svelte:window onkeydowncapture={onKeyDown} />

{#if open}
	<div transition:fadeFast|local class="fixed top-0 bottom-0 left-0 right-0 z-[9999]" role="dialog">
		<div
			class={classNames(
				'fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
				open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
			)}
		></div>

		<div class="fixed inset-0 z-10 overflow-y-auto">
			<div class="flex min-h-full items-center justify-center p-4">
				<div
					class={classNames(
						'relative transform overflow-hidden rounded-lg bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:my-8 sm:w-full sm:max-w-lg sm:p-6',
						open
							? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
							: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
					)}
				>
					<div class="flex">
						<div
							class="flex h-12 w-12 items-center justify-center rounded-full bg-blue-100 dark:bg-blue-800/50"
						>
							<RefreshCcw class="text-blue-700 dark:text-blue-300" />
						</div>
						<div class="ml-4 flex-1 text-left">
							<h3 class="text-lg font-medium text-primary">{title}</h3>
							<p class="mt-2 text-sm text-secondary">{body}</p>
						</div>
					</div>
					<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
						<Button
							on:click={() => onLoadLatest()}
							color="dark"
							size="sm"
							shortCut={{ Icon: CornerDownLeft, withoutModifier: true }}
							variant="accent"
						>
							<span class="min-w-20">{loadLabel}</span>
						</Button>
						<Button
							on:click={() => onKeepDraft()}
							variant="default"
							size="sm"
							shortCut={{ key: 'Esc', withoutModifier: true }}
						>
							Keep current draft
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
