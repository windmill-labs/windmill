<script lang="ts" generics="T">
	import type { Snippet } from 'svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import SessionStatusToken, { TOKEN_TRIGGER_CLASS } from './SessionStatusToken.svelte'
	import { isNewTabClick } from '$lib/navigation'

	// Shared shell for the session status-line popovers (Edits / Artifacts / Jobs):
	// one trigger token, one header, one keyboard-navigable row list — so styling
	// and behavior improvements land on all three segments at once.
	let {
		open = $bindable(),
		label,
		title,
		items,
		itemKey,
		onPick,
		itemHref,
		rowTitle,
		row,
		actions,
		footer,
		separatorAfter,
		customTrigger,
		ariaLabel,
		triggerClass = TOKEN_TRIGGER_CLASS,
		placement = 'top-start',
		closeOnOtherPopoverOpen = true,
		usePointerDownOutside = false,
		widthClass = 'w-80',
		maxHeightClass = 'max-h-[min(10rem,50vh)]'
	}: {
		open: boolean
		/** Trigger token text; also the default aria-label. */
		label: string
		/** Header line above the list ("… this session"). */
		title: string
		items: T[]
		itemKey: (item: T) => string
		/** Primary row activation; the popover closes itself first. */
		onPick: (item: T, e: MouseEvent) => void
		/** Makes a row a link opening this URL in a new tab, which middle and ⌘/Ctrl-click keep
		 * doing; on a plain click `onPick` may `preventDefault()` to open it in place instead. */
		itemHref?: (item: T) => string | undefined
		rowTitle?: (item: T) => string
		/** Content of the row's primary button. */
		row: Snippet<[T]>
		/** Trailing per-row action buttons (outside the primary button). */
		actions?: Snippet<[T]>
		/** Pinned below the scrolling list; give it `data-status-row` to join the
		 * arrow-key order as the last stop. */
		footer?: Snippet
		/** Closes a pinned group after this row, as a pane edge rather than a hairline —
		 * rows are otherwise unruled, so a 1px line would read as a row border. */
		separatorAfter?: (item: T, index: number) => boolean
		/** Replaces the default SessionStatusToken trigger. */
		customTrigger?: Snippet
		ariaLabel?: string
		/** Full class of the popover trigger (defaults to the token chrome). */
		triggerClass?: string
		placement?: 'top-start' | 'top-end'
		closeOnOtherPopoverOpen?: boolean
		usePointerDownOutside?: boolean
		widthClass?: string
		maxHeightClass?: string
	} = $props()

	function pick(item: T, e: MouseEvent) {
		open = false
		onPick(item, e)
	}

	// Roving focus over the rows' primary buttons.
	let listRoot = $state<HTMLDivElement | undefined>(undefined)

	function rowButtons(): HTMLElement[] {
		return listRoot
			? Array.from(
					listRoot.querySelectorAll<HTMLElement>('button[data-status-row], a[data-status-row]')
				)
			: []
	}

	function focusAt(index: number) {
		const buttons = rowButtons()
		if (buttons.length === 0) return
		const wrapped = ((index % buttons.length) + buttons.length) % buttons.length
		buttons[wrapped]?.focus()
	}

	function handleListKeydown(e: KeyboardEvent) {
		if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp' && e.key !== 'Home' && e.key !== 'End') {
			return
		}
		const buttons = rowButtons()
		if (buttons.length === 0) return
		// The active element may be a row's trailing action button: resolve to its
		// row so vertical navigation moves rows, not focusables.
		const active = document.activeElement as HTMLElement | null
		const current = buttons.findIndex((b) => b === active || b.parentElement?.contains(active))
		e.preventDefault()
		if (e.key === 'ArrowDown') focusAt(current < 0 ? 0 : current + 1)
		else if (e.key === 'ArrowUp') focusAt(current < 0 ? buttons.length - 1 : current - 1)
		else if (e.key === 'Home') focusAt(0)
		else if (e.key === 'End') focusAt(buttons.length - 1)
	}
</script>

<Popover
	bind:isOpen={open}
	{placement}
	enableFlyTransition
	{closeOnOtherPopoverOpen}
	{usePointerDownOutside}
	class={triggerClass}
	contentClasses="!bg-surface"
	triggerAttrs={{ 'aria-label': ariaLabel ?? label, 'aria-haspopup': 'dialog' }}
>
	{#snippet trigger()}
		{#if customTrigger}
			{@render customTrigger()}
		{:else}
			<SessionStatusToken {label} expanded={open} />
		{/if}
	{/snippet}

	{#snippet content()}
		<div class="flex {widthClass} flex-col text-xs">
			<div class="px-3 pt-2 pb-0.5 text-2xs text-hint">{title}</div>
			<!-- svelte-ignore a11y_no_static_element_interactions (keydown only routes arrows to the row buttons) -->
			<div class="flex min-h-0 flex-col" bind:this={listRoot} onkeydown={handleListKeydown}>
				<div role="list" class="{maxHeightClass} overflow-y-auto py-1">
					{#each items as item, index (itemKey(item))}
						{@const href = itemHref?.(item)}
						<div
							class="flex items-center gap-2 py-1 pl-3 pr-2 hover:bg-surface-hover focus-within:bg-surface-hover"
							role="listitem"
						>
							<svelte:element
								this={href ? 'a' : 'button'}
								type={href ? undefined : 'button'}
								role={href ? 'link' : 'button'}
								{href}
								target={href ? '_blank' : undefined}
								rel={href ? 'noopener' : undefined}
								data-status-row
								class="flex min-w-0 flex-1 items-center gap-2 text-left font-normal focus:outline-none text-[color:inherit]"
								title={rowTitle?.(item)}
								onclick={(e: MouseEvent) => {
									if (href && isNewTabClick(e)) return
									pick(item, e)
								}}
							>
								{@render row(item)}
							</svelte:element>
							{#if actions}
								<div class="flex shrink-0 items-center gap-1.5">
									{@render actions(item)}
								</div>
							{/if}
						</div>
						{#if separatorAfter?.(item, index)}
							<!-- Full-bleed and outside the row: a divider inside it would land in the
							     row's button and take its hover. role=presentation keeps the list's
							     children listitems for assistive tech. -->
							<div class="my-1 border-b-2 border-border-light" role="presentation"></div>
						{/if}
					{/each}
				</div>
				{#if footer}
					<div class="shrink-0 border-t py-1">
						{@render footer()}
					</div>
				{/if}
			</div>
		</div>
	{/snippet}
</Popover>
