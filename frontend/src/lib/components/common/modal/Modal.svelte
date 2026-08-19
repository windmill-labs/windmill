<script module lang="ts">
	/** One level of where you are inside a dialog. The last is the level you are on; the ones
	 *  before it carry the way back. */
	export type ModalTrailSegment = {
		label: string
		/** Absent on the level you are on, and on any ancestor that cannot be returned to. */
		onclick?: () => void
	}
</script>

<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'
	import { ChevronRight } from 'lucide-svelte'
	import { getOverlayHost, overlayHostActive } from '$lib/components/common/overlayHost.svelte'

	const bubble = createBubbler()
	import { createEventDispatcher, untrack } from 'svelte'
	import { fade } from 'svelte/transition'
	import Button from '../button/Button.svelte'
	import { twMerge } from 'tailwind-merge'
	import CloseButton from '../CloseButton.svelte'
	import Disposable from '../drawer/Disposable.svelte'
	import ConditionalPortal from '../drawer/ConditionalPortal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { chatState } from '$lib/components/copilot/chat/sharedChatState.svelte'

	interface Props {
		title: string
		open?: boolean
		class?: string
		style?: string
		cancelText?: string | undefined
		kind?: 'button' | 'X'
		/** Where you are inside the dialog, appended to the title as a breadcrumb. Levels below
		 * the title, so a dialog showing its own root passes nothing. The header is the one part
		 * of the surface that does not move, which is why the way back belongs in it rather than
		 * in a control each body places for itself. */
		trail?: ModalTrailSegment[]
		/** Make the dialog fill the height it is anchored to and lay its body out as a flex
		 * column, so content can size itself with `h-full` / `flex-1 min-h-0`. Off by default:
		 * the dialog otherwise hugs its content, and percentage heights inside it do not
		 * resolve (the centering wrapper is `min-h-full`, i.e. height:auto). */
		fillHeight?: boolean
		/** Force a minimum z-index base. Defaults to elevating above the AI chat
		 * side panel when it is open. Pass an explicit value to stack above other
		 * surfaces (e.g. a modal opened over the /sessions preview-pane editor). */
		minZIndex?: number
		settings?: import('svelte').Snippet
		children?: import('svelte').Snippet
		actions?: import('svelte').Snippet
	}

	let {
		title,
		open = $bindable(false),
		class: c = '',
		style = '',
		cancelText = undefined,
		kind = 'button',
		trail = undefined,
		fillHeight = false,
		minZIndex: minZIndexProp = undefined,
		settings,
		children: children_render,
		actions
	}: Props = $props()

	// Anchored to the enclosing pane when there is one (see overlayHost): portalled into it
	// and positioned against it, so the dialog covers that pane rather than the whole app.
	// Both halves are required — `absolute` resolves against the nearest positioned DOM
	// ancestor, which without the portal is whatever box the caller happens to sit in.
	const overlayHost = getOverlayHost()
	const hostEl = $derived(overlayHost?.el())
	const posClass = $derived(hostEl ? 'absolute' : 'fixed')
	const hostActive = overlayHostActive()

	const dispatch = createEventDispatcher()

	let disposable: Disposable | undefined = $state(undefined)

	// An explicit override wins; otherwise only elevate above the AI chat panel
	// when it's actually open — when chat is closed there's nothing at z-index
	// 1200 to stack above.
	const minZIndex = $derived(minZIndexProp ?? (chatState.size > 0 ? zIndexes.aiChat + 1 : 0))

	// Both `bind:open` and this $effect are needed: bind:open syncs the
	// boolean, while the effect calls openDrawer/closeDrawer to register
	// the disposable in the stacking system (same pattern as Drawer.svelte).
	$effect(() => {
		open
		untrack(() => {
			open ? disposable?.openDrawer() : disposable?.closeDrawer()
		})
	})

	function onKeyDown(event: KeyboardEvent) {
		// Hidden hosts stay mounted and still receive window keys — see overlayHost.
		if (!hostActive()) return
		// This dialog keeps Escape for itself (`preventEscape` below), so nothing else arbitrates
		// between it and whatever is stacked over it: without this, a drawer opened from inside the
		// dialog would take Escape and the dialog would act on it too.
		if (!disposable?.isTopmost()) return
		if (open) {
			switch (event.key) {
				case 'Enter':
					event.stopPropagation()
					event.preventDefault()
					dispatch('confirmed')
					break
				case 'Escape': {
					event.stopPropagation()
					event.preventDefault()
					// Inside a dialog that holds levels, Escape leaves the level rather than the
					// dialog: closing outright would throw away the surface someone navigated into,
					// which is the one thing they did not ask for. It still closes at the root.
					const back = trail && trail.length > 1 ? trail[trail.length - 2] : undefined
					if (back?.onclick) {
						back.onclick()
					} else {
						open = false
						dispatch('canceled')
					}
					break
				}
			}
		}
	}
	function fadeFast(node: HTMLElement) {
		return fade(node, { duration: 100 })
	}
</script>

<svelte:window onkeydowncapture={onKeyDown} />

<Disposable bind:open bind:this={disposable} preventEscape {minZIndex}>
	{#snippet children({ zIndex })}
		<!-- Always portalled, as Drawer is: to the enclosing pane when one claims it, to `body`
		     otherwise. Rendered in place it inherits whatever the caller happens to sit inside, and
		     one `transform`, `filter` or `overflow` anywhere above it confines a dialog that is
		     meant to cover the app — the nav rail then paints over it, and its own edges are clipped
		     to a box it never asked for. -->
		<ConditionalPortal condition target={hostEl} class={hostEl ? 'contents' : undefined}>
			{#if open}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<div
					onclick={() => (open = false)}
					transition:fadeFast|local
					class="{posClass} top-0 bottom-0 left-0 right-0"
					style="z-index: {zIndex}"
					role="dialog"
					tabindex="-1"
				>
					<div
						class={twMerge(
							posClass,
							'inset-0 bg-gray-500 bg-opacity-75 transition-opacity',
							open ? 'ease-out duration-300 opacity-100' : 'ease-in duration-200 opacity-0'
						)}
					></div>

					<div class="{posClass} inset-0 z-10 overflow-y-auto">
						<div
							class="flex {fillHeight ? 'h-full' : 'min-h-full'} items-center justify-center p-4"
						>
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								onclick={stopPropagation(bubble('click'))}
								class={twMerge(
									'relative transform overflow-hidden rounded-md bg-surface px-4 pt-5 pb-4 text-left shadow-xl transition-all sm:w-full sm:max-w-lg sm:p-6',
									// The margins are what keeps a content-sized dialog off the viewport edges; a
									// filling one takes its inset from the wrapper's padding instead.
									fillHeight ? 'h-full flex flex-col' : 'sm:my-8',
									c,
									open
										? 'ease-out duration-300 opacity-100 translate-y-0 sm:scale-100'
										: 'ease-in duration-200 opacity-0 translate-y-4 sm:translate-y-0 sm:scale-95'
								)}
								{style}
							>
								{#if kind == 'X'}
									<div class="absolute top-4 right-4"
										><CloseButton on:close={() => (open = false)} /></div
									>
								{/if}
								<div class="flex {fillHeight ? 'flex-1 min-h-0' : ''}">
									<!-- min-w-0: without it this flex item takes its content's min-content width and
									     stretches the modal past its max-width instead of letting content shrink. -->
									<div class="text-left flex-1 min-w-0 {fillHeight ? 'flex flex-col min-h-0' : ''}">
										<!-- pr-8 under `kind="X"`: the close button is positioned against the
										     dialog rather than laid out in this row, so a long trail would
										     otherwise run under it. -->
										<div
											class="flex flex-row items-center justify-between gap-2 min-w-0 {kind === 'X'
												? 'pr-8'
												: ''}"
										>
											<nav
												aria-label="Breadcrumb"
												class="flex flex-row items-center gap-1 min-w-0 text-lg font-semibold"
											>
												<h3 class="text-emphasis shrink-0">{title}</h3>
												{#each trail ?? [] as segment, i (i)}
													<ChevronRight size={18} class="text-tertiary shrink-0" />
													{#if segment.onclick}
														<button
															type="button"
															class="text-secondary hover:text-emphasis hover:underline truncate"
															onclick={segment.onclick}
														>
															{segment.label}
														</button>
													{:else}
														<span class="text-emphasis truncate" aria-current="page">
															{segment.label}
														</span>
													{/if}
												{/each}
											</nav>
											{@render settings?.()}
										</div>

										<div class="mt-4 text-sm text-primary {fillHeight ? 'flex-1 min-h-0' : ''}">
											{@render children_render?.()}
										</div>
									</div>
								</div>
								{#if kind == 'button'}
									<div class="flex items-center space-x-2 flex-row-reverse space-x-reverse mt-4">
										{@render actions?.()}
										<Button
											on:click={() => {
												dispatch('canceled')
												open = false
											}}
											color="light"
											size="sm"
										>
											{cancelText ?? 'Cancel'}
										</Button>
									</div>
								{/if}
							</div>
						</div>
					</div>
				</div>
			{/if}
		</ConditionalPortal>
	{/snippet}
</Disposable>
