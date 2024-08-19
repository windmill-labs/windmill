<script>
	import { twMerge } from 'tailwind-merge'
</script>

<button
	on:click
	class={twMerge(
		'bg-surface hover:bg-surface-hover rounded-full border text-xs',
		'w-6 h-6',
		'relative center-center'
	)}
>
	<slot />
</button>
