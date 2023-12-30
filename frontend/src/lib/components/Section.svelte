<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { AlertTriangle, ChevronDown, ChevronRight } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'

	export let label: string | undefined = undefined
	export let tooltip: string | undefined = undefined
	export let eeOnly = false

	export let collapsable: boolean = false
	let collapsed: boolean = true
</script>

<div class="w-full">
	<div class="flex flex-row justify-between items-center mb-2">
		<h2 class="text-base font-semibold flex flex-row gap-1">
			{#if collapsable}
				<button class="flex items-center gap-1" on:click={() => (collapsed = !collapsed)}>
					{#if collapsed}
						<ChevronRight size={16} />
					{:else}
						<ChevronDown size={16} />
					{/if}
					{label}
				</button>
			{:else}
				{label}
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
	</div>
	<div class={collapsable && collapsed ? `hidden ${$$props.class}` : `${$$props.class}`}>
		<slot />
	</div>
</div>
