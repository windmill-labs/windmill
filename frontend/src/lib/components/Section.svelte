<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { AlertTriangle, ChevronDown, ChevronRight } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	export let label: string | undefined = undefined
	export let tooltip: string | undefined = undefined
	export let eeOnly = false
	export let small: boolean = false

	export let subsection: boolean = false

	export let collapsable: boolean = false
	let collapsed: boolean = true
</script>

<div class="w-full">
	<div class="flex flex-row justify-between items-start mb-2">
		<h2
			class={twMerge(
				subsection ? 'font-normal' : 'font-semibold',
				subsection ? 'text-secondary' : 'text-primary',
				'flex flex-row items-center gap-1',
				small ? 'text-sm' : 'text-base'
			)}
		>
			{label}
			{#if collapsable}
				<button class="flex items-center gap-1" on:click={() => (collapsed = !collapsed)}>
					{#if collapsed}
						<ChevronRight size={16} />
					{:else}
						<ChevronDown size={16} />
					{/if}
				</button>
			{/if}

			<slot name="header" />
			{#if tooltip}
				<Tooltip>{tooltip}</Tooltip>
			{/if}
			{#if eeOnly}
				{#if !$enterpriseLicense}
					<div class="flex text-xs items-center gap-1 text-yellow-500 whitespace-nowrap ml-8">
						<AlertTriangle size={16} />
						EE only <Tooltip>Enterprise Edition only feature</Tooltip>
					</div>
				{/if}
			{/if}
		</h2>
		<slot name="action" />
		{#if collapsable && collapsed}
			<slot name="badge" />
		{/if}
	</div>

	<div class={collapsable && collapsed ? `hidden ${$$props.class}` : `${$$props.class}`}>
		<slot />
	</div>
</div>
