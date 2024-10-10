<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let disabled: boolean = false
</script>

<button
	on:click
	class={twMerge(
		'bg-surface-secondary hover:bg-surface-hover rounded-md border text-xs',
		'w-6 h-6',
		'relative center-center',
		disabled ? 'cursor-not-allowed bg-surface-disabled' : 'cursor-pointer'
	)}
>
	<slot />
</button>
