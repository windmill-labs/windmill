<script lang="ts">
	import { twMerge } from 'tailwind-merge'

	export let kbdClass = ''
	export let small = false
	export let isModifier: boolean = false

	if (small) {
		kbdClass = twMerge(
			kbdClass,
			'!text-[10px]  px-1',
			isModifier ? '!text-lg ' : 'text-xs',
			'leading-none'
		)
	} else {
		kbdClass += ' !text-xs px-1.5'
	}
</script>

<span
	class={twMerge(
		$$props.class,
		small ? 'h-4  center-center' : '',
		'ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse'
	)}
>
	<kbd class={kbdClass}>
		<slot />
	</kbd>
</span>
