<script lang="ts">
	import { classNames } from '$lib/utils'
	import ConditionalPortal from '$lib/components/common/drawer/ConditionalPortal.svelte'
	import { getOverlayHost, overlayHostActive } from '$lib/components/common/overlayHost.svelte'
	import { createEventDispatcher, type Snippet } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { AlertTriangle, CornerDownLeft, Info, Loader2, RefreshCcw } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	type Props = {
		title: string
		confirmationText: string
		keyListen?: boolean
		loading?: boolean
		/** Blocks confirming (button and Enter) while a required choice in `children` is unmade. */
		confirmDisabled?: boolean
		open?: boolean
		type?: 'danger' | 'reload' | 'info'
		showIcon?: boolean
		id?: string
		trashbin?: boolean
		/** Tailwind z-index class for the modal root. Override to stack this modal
		 * above another modal that's already open (both default to `z-[9999]`). */
		zIndexClass?: string
		/** Render into `body` instead of where this component sits. Needed when an ancestor
		 * creates a stacking context the dialog has to escape — a drawer paints over the page
		 * whatever the dialog's z-index, and a `transform`, `filter` or `overflow` on the way
		 * up confines it. Off by default: it moves the dialog out of its DOM position, so opt
		 * in per call site rather than assuming every caller wants it. */
		alwaysPortal?: boolean
		children?: Snippet
		onConfirmed?: () => void | Promise<void>
		onCanceled?: () => void
	}

	const {
		title,
		confirmationText,
		keyListen = true,
		loading = false,
		confirmDisabled = false,
		open = false,
		type: _type,
		showIcon = true,
		id,
		trashbin = false,
		zIndexClass = 'z-[9999]',
		alwaysPortal = false,
		children,
		onConfirmed,
		onCanceled
	}: Props = $props()
	const type = $derived(_type ?? 'danger')

	// Anchored to the enclosing pane when there is one (see overlayHost): portalled into it
	// and positioned against it, so the dialog covers that pane rather than the whole app.
	// Both halves are required — `absolute` resolves against the nearest positioned DOM
	// ancestor, which without the portal is whatever box the caller happens to sit in.
	const overlayHost = getOverlayHost()
	const hostEl = $derived(overlayHost?.el())
	const posClass = $derived(hostEl ? 'absolute' : 'fixed')

	const hostActive = overlayHostActive()

	const dispatch = createEventDispatcher()

	function onKeyDown(event: KeyboardEvent) {
		// Hidden hosts stay mounted and still receive window keys — see overlayHost.
		if (!hostActive()) return
		if (open && keyListen) {
			// Only intercept Enter/Escape (without modifiers) so shortcuts like
			// Cmd/Ctrl+C and Cmd/Ctrl+V keep working inside the modal.
			if (event.metaKey || event.ctrlKey || event.altKey) {
				return
			}
			const popover = (event.target as HTMLElement | null)?.closest?.('[data-popover]')
			// Content carries no `aria-controls`; a trigger's resolves only while its content is
			// mounted, which is the only reliable open/closed signal — the trigger's own aria state
			// is stale because visibility is driven outside melt.
			const controls = popover?.getAttribute('aria-controls')
			const popoverOpen = !!popover && (!controls || !!document.getElementById(controls))

			switch (event.key) {
				// Both keys are gated on the same state as the button they stand for, which is why
				// they swallow the event first and only then decide. Ungated, Enter re-enters an
				// in-flight confirm and Escape dismisses the modal out from under one — leaving the
				// action to finish against a caller that believes it was cancelled.
				case 'Enter':
					// A popover needs Enter both to open from its trigger and to choose from its
					// content, so leave it alone whether or not it is open.
					if (popover) return
					event.stopPropagation()
					event.preventDefault()
					if (loading || confirmDisabled) break
					dispatch('confirmed')
					onConfirmed?.()
					break
				case 'Escape':
					// Only an open popover has something to dismiss; on a closed trigger Escape is
					// still the dialog's.
					if (popoverOpen) return
					event.stopPropagation()
					event.preventDefault()
					if (loading) break
					dispatch('canceled')
					onCanceled?.()
					break
			}
		}
	}
	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}

	const theme = {
		danger: {
			Icon: AlertTriangle,
			color: 'red',
			classes: {
				icon: 'text-red-500 dark:text-red-400',
				iconWrapper: 'bg-red-100 dark:bg-red-800/50'
			}
		},

		reload: {
			Icon: RefreshCcw,
			color: 'dark',
			classes: {
				icon: 'text-blue-700 dark:text-blue-300',
				iconWrapper: 'bg-blue-100 dark:bg-blue-800/50'
			}
		},

		// Neutral, affirmative confirmation (non-destructive) — e.g. proceeding with
		// an import. The confirm button stays a plain accent (see `destructive` below).
		info: {
			Icon: Info,
			color: 'blue',
			classes: {
				icon: 'text-blue-700 dark:text-blue-300',
				iconWrapper: 'bg-blue-100 dark:bg-blue-800/50'
			}
		}
	} satisfies { [type in typeof type]: any }
	const Icon = $derived(theme[type].Icon ?? AlertTriangle)
</script>

<svelte:window onkeydowncapture={onKeyDown} />

<ConditionalPortal
	condition={alwaysPortal || !!hostEl}
	target={hostEl}
	class={hostEl ? 'contents' : undefined}
>
	{#if open}
		<div
			transition:fadeFast|local
			class={twMerge(posClass, 'top-0 bottom-0 left-0 right-0', zIndexClass)}
			role="dialog"
			{id}
		>
			<div
				class={classNames(
					posClass,
					'inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
					open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
				)}
			></div>

			<div class="{posClass} inset-0 z-10 overflow-y-auto">
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
							{#if showIcon}
								<div
									class={`flex h-12 w-12 items-center justify-center rounded-full ${theme[type].classes.iconWrapper}`}
								>
									<Icon class={theme[type].classes.icon} />
								</div>
							{/if}
							<div class={twMerge('ml-0 text-left flex-1 ', showIcon ? 'ml-4' : '')}>
								<h3 class="text-lg font-medium text-primary">
									{title}
								</h3>
								<div class="mt-2 text-sm text-secondary">
									{@render children?.()}
								</div>
								{#if trashbin}
									<p class="mt-3 text-xs text-tertiary"
										>This item will be moved to the trashbin and can be restored by a workspace
										admin within 3 days.</p
									>
								{/if}
							</div>
						</div>
						<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
							<Button
								disabled={loading || confirmDisabled}
								on:click={() => (dispatch('confirmed'), onConfirmed?.())}
								color={theme[type].color}
								size="sm"
								shortCut={{ Icon: CornerDownLeft, hide: !keyListen, withoutModifier: true }}
								variant="accent"
								destructive={type === 'danger'}
							>
								{#if loading}
									<Loader2 class="animate-spin" />
								{/if}
								<span class="min-w-20">{confirmationText} </span>
							</Button>
							<Button
								disabled={loading}
								on:click={() => (dispatch('canceled'), onCanceled?.())}
								variant="default"
								size="sm"
								shortCut={{ key: 'Esc', hide: !keyListen, withoutModifier: true }}
							>
								Cancel
							</Button>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
</ConditionalPortal>
