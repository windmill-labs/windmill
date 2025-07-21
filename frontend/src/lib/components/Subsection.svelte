<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { AlertTriangle, ChevronDown, ChevronRight } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		label?: string | undefined
		tooltip?: string | undefined
		eeOnly?: boolean
		collapsable?: boolean
		collapsed?: boolean
		openInitially?: boolean
		headless?: boolean
		class?: string | undefined
		header?: import('svelte').Snippet
		action?: import('svelte').Snippet
		badge?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		label = undefined,
		tooltip = undefined,
		eeOnly = false,
		collapsable = false,
		collapsed = $bindable(true),
		openInitially = false,
		headless = false,
		class: clazz = undefined,
		header,
		action,
		badge,
		children
	}: Props = $props()

	if (openInitially && collapsable && collapsed) {
		collapsed = false
	}
</script>

<div class="w-full">
	{#if !headless}
		<div class="flex flex-row justify-between items-center mb-1">
			<h3 class={twMerge('font-semibold flex flex-row items-center gap-2', 'text-sm')}>
				{#if collapsable}
					<button class="flex items-center gap-1" onclick={() => (collapsed = !collapsed)}>
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

				{@render header?.()}
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
			</h3>
			{@render action?.()}
			{#if collapsable && collapsed}
				{@render badge?.()}
			{/if}
		</div>
	{/if}
	<div class={collapsable && collapsed ? `hidden ${clazz ?? ''}` : `${clazz ?? ''}`}>
		{@render children?.()}
	</div>
</div>
