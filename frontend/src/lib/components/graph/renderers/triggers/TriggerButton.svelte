<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	interface Props {
		selected: boolean
		children?: import('svelte').Snippet
		onclick?: (...args: any[]) => any
	}

	let { selected, children, onclick = undefined }: Props = $props()
</script>

<button
	onclick={(e) => onclick?.(e)}
	class={twMerge(
		'hover:bg-surface-hover rounded-md border text-xs w-[23px] h-[23px] relative center-center cursor-pointer bg-surface',
		selected ? 'outline-1 outline-tertiary outline' : 'outline-0'
	)}
>
	{@render children?.()}
</button>
