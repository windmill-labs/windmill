<script lang="ts">
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import type { PopoverPlacement } from './Popover.model'
	import Portal from '$lib/components/Portal.svelte'
	import { twMerge } from 'tailwind-merge'

	import { ExternalLink } from 'lucide-svelte'
	import { untrack } from 'svelte'

	interface Props {
		placement?: PopoverPlacement
		notClickable?: boolean
		popupClass?: string
		disablePopup?: boolean
		disappearTimeout?: number
		appearTimeout?: number
		documentationLink?: string | undefined
		style?: string | undefined
		forceOpen?: boolean
		class?: string
		children?: import('svelte').Snippet
		text?: import('svelte').Snippet
		onClick?: () => void
	}

	let {
		placement = 'bottom-end',
		notClickable = false,
		popupClass = '',
		disablePopup = false,
		disappearTimeout = 100,
		appearTimeout = 300,
		documentationLink = undefined,
		style = undefined,
		forceOpen = false,
		class: classNames = '',
		children,
		text,
		onClick
	}: Props = $props()

	const [popperRef, popperContent] = createPopperActions({ placement })

	const popperOptions: PopperOptions<{}> = {
		placement,
		strategy: 'fixed',
		modifiers: [
			{ name: 'offset', options: { offset: [8, 8] } },
			{
				name: 'arrow',
				options: {
					padding: 10
				}
			}
		]
	}

	let showTooltip = $state(false)
	let timeout: NodeJS.Timeout | undefined = undefined
	let inTimeout: NodeJS.Timeout | undefined = undefined

	function open() {
		clearTimeout(timeout)
		if (appearTimeout == 0) {
			showTooltip = true
		} else {
			inTimeout = setTimeout(() => (showTooltip = true), appearTimeout)
		}
	}
	function close() {
		inTimeout && clearTimeout(inTimeout)
		inTimeout = undefined
		timeout = setTimeout(() => (showTooltip = false), disappearTimeout)
	}

	$effect(() => {
		;[forceOpen]
		untrack(() => (forceOpen ? open() : close()))
	})
</script>

{#if notClickable}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<span {style} use:popperRef onmouseenter={open} onmouseleave={close} class={classNames}>
		{@render children?.()}
	</span>
{:else}
	<button
		{style}
		use:popperRef
		onmouseenter={open}
		onmouseleave={close}
		onclick={onClick}
		class={classNames}
	>
		{@render children?.()}
	</button>
{/if}
{#if showTooltip && !disablePopup}
	<Portal name="popover">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			use:popperContent={popperOptions}
			onmouseenter={open}
			onmouseleave={close}
			class={twMerge(
				'z-[5001] py-2 px-3 rounded-md text-sm font-normal !text-gray-300 bg-gray-800 whitespace-normal text-left',
				popupClass
			)}
		>
			<div class="max-w-sm break-words">
				{@render text?.()}
				{#if documentationLink}
					<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
						<div class="flex flex-row gap-2 mt-4">
							See documentation
							<ExternalLink size="16" />
						</div>
					</a>
				{/if}
			</div>
		</div>
	</Portal>
{/if}
