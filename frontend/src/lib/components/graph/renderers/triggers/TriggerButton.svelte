<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let selected: boolean
</script>

<button
	on:click
	class={twMerge(
		'hover:bg-surface-hover rounded-md border text-xs w-[23px] h-[23px] relative center-center cursor-pointer bg-surface',
		selected ? 'outline-1 outline-tertiary outline' : 'outline-0'
	)}
>
	<slot />
</button>
