<script>
	import { twMerge } from 'tailwind-merge'
</script>

<button
	on:click
	class={twMerge(
		'bg-surface-secondary hover:bg-surface-hover rounded-md border text-xs',
		'w-6 h-6',
		'relative center-center'
	)}
>
	<slot />
</button>
