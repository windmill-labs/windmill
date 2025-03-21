<script lang="ts">
	import { classes } from '$lib/components/common/alert/model'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher()

	export let hidden: boolean = false
</script>

<div
	class={twMerge(
		'rounded-md flex flex-row gap-2 items-center px-2 w-fit',
		classes['info'].bgClass,
		hidden ? 'hidden' : '',
		$$props.class
	)}
>
	<p class={twMerge(classes['info'].descriptionClass, 'text-xs px-2')}>
		<slot />
	</p>
	<div class="flex gap-1">
		<slot name="action" />
		<Button color="light" size="xs2" startIcon={{ icon: X }} on:click={() => dispatch('close')} />
	</div>
</div>
