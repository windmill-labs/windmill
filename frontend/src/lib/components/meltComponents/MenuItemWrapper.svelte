<script lang="ts">
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuElements } from '@melt-ui/svelte'

	// Registers arbitrary content (not just a button/link, unlike MenuItem) as a melt menu
	// item, so it participates in the menu's roving focus/highlight. Useful for custom rows
	// that contain their own interactive controls (e.g. a slider).
	interface Props {
		item: MenubarMenuElements['item']
		class?: string | undefined
		onClick?: (event: MouseEvent) => void
		onKeydown?: (event: KeyboardEvent) => void
		children?: import('svelte').Snippet
	}

	let { item, class: className = undefined, onClick, onKeydown, children }: Props = $props()
</script>

<div
	use:melt={$item}
	role="menuitem"
	tabindex="-1"
	class={className}
	onclick={onClick}
	onkeydown={onKeydown}
>
	{@render children?.()}
</div>
