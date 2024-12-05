<script lang="ts">
	import { classNames } from '$lib/utils'
	// import { MenuItem } from '@rgossiaux/svelte-headlessui'

	import { tutorialsToDo } from '$lib/stores'
	import { CheckCircle, Circle } from 'lucide-svelte'

	export let label: string
	export let index: number
	export let disabled: boolean = false
</script>

<!-- <MenuItem on:click {disabled}>
	<div
		class={classNames(
			'flex flex-row items-center text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold',
			disabled ? 'text-disabled' : 'text-primary'
		)}
	>
		{#if $tutorialsToDo.includes(index)}
			<Circle size={16} />
		{:else}
			<CheckCircle size={16} color="green" />
		{/if}
		<div class="flex flex-row justify-between items-center w-full">
			{label}
			<slot />
		</div>
	</div>
</MenuItem> -->
