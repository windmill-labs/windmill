<script lang="ts">
	import { classNames } from '$lib/utils'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

	import { tutorialsToDo } from '$lib/stores'
	import { CheckCircle, Circle } from 'lucide-svelte'

	export let label: string
	export let index: number
</script>

<MenuItem on:click>
	<div
		class={classNames(
			'text-primary flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
		)}
	>
		{#if $tutorialsToDo.includes(index)}
			<Circle size={16} />
		{:else}
			<CheckCircle size={16} color="green" />
		{/if}
		{label}
	</div>
</MenuItem>
