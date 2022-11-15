<!-- svelte-ignore a11y-click-events-have-key-events -->
<div on:click class="py-1">
	<span
		class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
		role="menuitem"
		tabindex="-1"
	>
		<slot />
	</span>
</div>
