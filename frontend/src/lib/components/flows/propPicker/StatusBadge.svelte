<script lang="ts">
	import { classes } from '$lib/components/common/alert/model'
	import { twMerge } from 'tailwind-merge'

	export let hidden: boolean = false
</script>

<div
	class={twMerge(
		'rounded-md flex flex-row gap-1 items-center px-1 w-fit',
		classes['info'].bgClass,
		hidden ? 'hidden' : '',
		$$props.class
	)}
>
	<p class={twMerge(classes['info'].descriptionClass, 'text-xs px-2')}>
		<slot />
	</p>

	<slot name="action" />
</div>
