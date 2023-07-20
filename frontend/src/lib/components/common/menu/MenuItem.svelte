<!-- svelte-ignore a11y-click-events-have-key-events -->
<div on:click class="py-1">
	<span
		class="text-secondary block px-4 py-2 text-sm hover:bg-surface-hover hover:text-primary"
		role="menuitem"
		tabindex="-1"
	>
		<slot />
	</span>
</div>
