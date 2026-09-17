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
	import { ChevronLeft, ChevronRight } from 'lucide-svelte'
	import {
		getOverlayHost,
		overlayHostActive,
		setTopmostSurface
	} from '$lib/components/common/overlayHost.svelte'

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
		/** Where you are inside the dialog: the whole path, the dialog's own root first. Below the
		 * root the last level becomes the heading and the ones above it the line under it, with a
		 * back control for the nearest. A dialog at its root passes nothing (or one level) and
		 * keeps its plain title; ancestors with an `onclick` are the way back, which Escape also
		 * takes. */
		trail?: ModalTrailSegment[]
		/** A line under the title saying what the dialog is for; in the header so it does not
		 * scroll away with the body. */
		description?: string
		/** The body holds pages that are laid over each other rather than stacked, so the header
		 * keeps its own height and only the pages move. Requires `fillHeight`: pages are absolutely
		 * positioned and need a definite height to fill. Pair with `PagedContent`, which does the
		 * laying over; this only makes room for it. */
		paginated?: boolean
		/**
		 * Whether Enter confirms the dialog. On by default, which is right for a form. Turn it off
		 * where the body is a surface with its own meaning for Enter: the handler runs at `window`
		 * in the capture phase and stops propagation, so while it is on nothing inside the dialog
		 * can see the key at all.
		 */
		enterConfirms?: boolean
		/** Make the dialog fill the height it is anchored to and lay its body out as a flex
		 * column, so content can size itself with `h-full` / `flex-1 min-h-0`. Off by default:
		 * the dialog otherwise hugs its content, and percentage heights inside it do not
		 * resolve (the centering wrapper is `min-h-full`, i.e. height:auto). */
		fillHeight?: boolean
		/** Force a minimum z-index base. Defaults to elevating above the AI chat
		 * side panel when it is open. Pass an explicit value to stack above other
		 * surfaces (e.g. a modal opened over the /sessions preview-pane editor). */
		minZIndex?: number
		/** Rendered against the dialog's own name, wherever that name is: the heading at the root,
		 * the first level of the way back below it. */
		titleBadge?: import('svelte').Snippet
		/** Rendered against the level you are on, which below the root is the heading. Nothing at
		 * the root, where that level is the dialog and `titleBadge` already names it. */
		levelBadge?: import('svelte').Snippet
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
		description = undefined,
		fillHeight = false,
		paginated = false,
		enterConfirms = true,
		minZIndex: minZIndexProp = undefined,
		titleBadge,
		levelBadge,
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

	// A trail of one level is the dialog at its root, which the plain title already shows.
	const crumbs = $derived(trail && trail.length > 1 ? trail : undefined)
	// The level you are on, which is the heading below the root.
	const current = $derived(crumbs?.[crumbs.length - 1])
	// The level under it: what Escape and the back control return to.
	const back = $derived(crumbs?.[crumbs.length - 2])

	const dispatch = createEventDispatcher()

	let disposable: Disposable | undefined = $state(undefined)

	// An explicit override wins; otherwise only elevate above the AI chat panel
	// when it's actually open — when chat is closed there's nothing at z-index
	// 1200 to stack above.
	const minZIndex = $derived(minZIndexProp ?? (chatState.size > 0 ? zIndexes.aiChat + 1 : 0))

	// So content in the body can tell a key meant for this dialog from one meant for whatever was
	// opened over it. Read lazily: `disposable` is bound after this runs.
	setTopmostSurface(() => disposable?.isTopmost() ?? true)

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
		// `preventEscape` below keeps Escape for this dialog, so nothing else arbitrates between it
		// and an overlay stacked over it (a drawer opened from inside): ask before acting.
		if (!disposable?.isTopmost()) return
		if (open) {
			switch (event.key) {
				case 'Enter':
					if (!enterConfirms) break
					event.stopPropagation()
					event.preventDefault()
					dispatch('confirmed')
					break
				case 'Escape': {
					event.stopPropagation()
					event.preventDefault()
					// Inside a dialog that holds levels, Escape leaves the level, not the dialog; it
					// still closes at the root.
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

<!-- The heading names the level you are on; where it sits within the dialog goes under it. -->
<!-- `oneLine` for a level of the trail, which shares its row with the way back and the actions.
     The dialog's own title wraps instead, as it did before there was a trail: it is the whole row,
     and titles carrying a path or a sentence are long by nature. -->
{#snippet heading(label: string, badge: import('svelte').Snippet | undefined, oneLine: boolean)}
	<h3
		class="text-emphasis text-lg font-semibold leading-7 flex items-center gap-1 {oneLine
			? 'min-w-0'
			: ''}"
	>
		<span class={oneLine ? 'truncate' : ''}>{label}</span>
		{@render badge?.()}
	</h3>
{/snippet}

<Disposable bind:open bind:this={disposable} preventEscape {minZIndex}>
	{#snippet children({ zIndex })}
		<!-- Always portalled, as Drawer is: rendered in place, any `transform`, `filter` or
		     `overflow` on an ancestor confines the dialog and the nav rail paints over it. -->
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
										<!-- pr-8 under `kind="X"`: the close button is absolutely positioned, so a long
										     heading or the header actions would otherwise run under it. -->
										<div
											class="flex flex-row items-start justify-between gap-2 min-w-0 {kind === 'X'
												? 'pr-8'
												: ''}"
										>
											{#if crumbs && current}
												<div class="flex flex-row items-start gap-1 min-w-0">
													{#if back?.onclick}
														<!-- Pulled left so the heading lands about where the title sits at the
														     root, and up so it reads against that heading rather than against
														     the two lines together. -->
														<Button
															variant="subtle"
															unifiedSize="sm"
															iconOnly
															startIcon={{ icon: ChevronLeft }}
															title="Back to {back.label}"
															onClick={back.onclick}
															wrapperClasses="shrink-0 -ml-1 -mt-0.5"
														/>
													{/if}
													<div class="flex flex-col min-w-0">
														{@render heading(current.label, levelBadge, true)}
														<!-- The way back in full: the control above carries one level of it. -->
														<nav
															aria-label="Breadcrumb"
															class="flex flex-row items-center gap-0.5 min-w-0 text-xs text-secondary"
														>
															{#each crumbs.slice(0, -1) as segment, i (i)}
																{#if i > 0}
																	<span class="flex shrink-0" in:fade={{ duration: 150 }}>
																		<ChevronRight size={12} class="text-tertiary shrink-0" />
																	</span>
																{/if}
																{#if segment.onclick}
																	<Button
																		variant="subtle"
																		unifiedSize="2xs"
																		onClick={segment.onclick}
																		wrapperClasses="min-w-0 shrink"
																		btnClasses="!px-0 !font-normal !text-xs text-secondary hover:text-emphasis hover:underline hover:!bg-transparent min-w-0"
																	>
																		<span class="truncate">{segment.label}</span>
																	</Button>
																{:else}
																	<span class="truncate" in:fade={{ duration: 150 }}
																		>{segment.label}</span
																	>
																{/if}
																{#if i === 0}
																	{@render titleBadge?.()}
																{/if}
															{/each}
														</nav>
													</div>
												</div>
											{:else}
												{@render heading(title, titleBadge, false)}
											{/if}
											{@render settings?.()}
										</div>

										{#if description}
											<p class="mt-1 text-xs text-secondary">{description}</p>
										{/if}

										<!-- `mt-1` when paginated: a page carries its own description as its first line,
										     and it belongs where the dialog's own sat, right under the title. -->
										<div
											class="{paginated ? 'mt-1' : 'mt-4'} text-sm text-primary {fillHeight
												? 'flex-1 min-h-0'
												: ''}"
										>
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
