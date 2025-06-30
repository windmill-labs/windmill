<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	interface Props {
		aiId?: string | undefined
		aiDescription?: string | undefined
		href?: string | undefined
		disabled?: boolean
		target?: string | undefined
		item: MenubarMenuElements['item']
		class?: string | undefined
		onClick?: (event: MouseEvent) => void
		children?: import('svelte').Snippet
		onFocusIn?: (event: FocusEvent) => void
		onFocusOut?: (event: FocusEvent) => void
	}

	let {
		aiId = undefined,
		aiDescription = undefined,
		href = undefined,
		disabled = false,
		target = undefined,
		item,
		class: classNames = undefined,
		children,
		onClick,
		onFocusIn,
		onFocusOut
	}: Props = $props()

	let aRef: HTMLAnchorElement | undefined = $state(undefined)
	let buttonRef: HTMLButtonElement | undefined = $state(undefined)
</script>

{#if href}
	<a
		bind:this={aRef}
		use:melt={$item}
		use:triggerableByAI={{
			id: aiId,
			description: aiDescription,
			callback: () => {
				aRef?.click()
			}
		}}
		{href}
		class={classNames}
		role="menuitem"
		aria-disabled={disabled}
		tabindex={disabled ? -1 : undefined}
		{target}
		onfocusin={onFocusIn}
		onfocusout={onFocusOut}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={buttonRef}
		onclick={onClick}
		use:melt={$item}
		use:triggerableByAI={{
			id: aiId,
			description: aiDescription,
			callback: () => {
				buttonRef?.click()
			}
		}}
		{disabled}
		class={classNames}
		role="menuitem"
		onfocusin={onFocusIn}
		onfocusout={onFocusOut}
	>
		{@render children?.()}
	</button>
{/if}
