<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import type { HomeHeaderMenu } from './homeHeaderMenuState.svelte'

	let { menu }: { menu: HomeHeaderMenu } = $props()
</script>

<!-- Last in the bar whatever else the page registers: a menu of side trips belongs after the
     buttons that act on what is on screen. -->
<PageHeaderContent actions={homeMenu} actionsOrder={100} />

{#snippet homeMenu()}
	<!-- An ellipsis that opens an empty menu is a button that does nothing. -->
	{#if menu.items.length > 0}
		<!-- The flash rides the ellipsis itself, not the row: with side trips beside it the row is
		     several controls wide, and marking all of them names none of them. -->
		<span class="relative inline-flex">
			{#if menu.pulsing}
				<!-- `opacity-0` at rest: the keyframes carry no fill mode, so at the end of the
				     animation the element returns to its own opacity. At 1 that is the flash coming
				     back at full strength until the unmount timer catches up. -->
				<span
					aria-hidden="true"
					class="pointer-events-none absolute -inset-1 rounded-lg bg-blue-500/30 opacity-0 animate-fade-out"
				></span>
			{/if}
			<DropdownV2 placement="bottom-end" size="sm" items={menu.items} />
		</span>
	{/if}
{/snippet}
