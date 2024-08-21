<script lang="ts">
	import { Tab } from '@rgossiaux/svelte-headlessui'
	import { twMerge } from 'tailwind-merge'

	import Toggle from '$lib/components/Toggle.svelte'

	export let label: string | undefined = undefined
	export let tooltip: string | undefined = undefined
	export let disabled: boolean = false
	export let small = false
	export let light = false
	export let checked: boolean = false

	export let id: string | undefined = undefined
</script>

<div {id} class="flex">
	<Tab
		{disabled}
		class={twMerge(
			' rounded-md transition-all text-xs flex gap-1 flex-row items-center',
			small ? 'px-1.5 py-0.5 text-2xs' : 'px-2 py-1',
			light ? 'font-medium' : '',
			'bg-surface-secondary ',
			$$props.class
		)}
	>
		<Toggle
			size="xs"
			options={{
				right: label,
				rightTooltip: tooltip
			}}
			textClass="text-2xs whitespace-nowrap white !w-full"
			bind:checked
		/>
	</Tab>
</div>
