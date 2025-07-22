<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { AlertTriangle, ChevronRight } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'

	interface Props {
		label?: string | undefined
		tooltip?: string | undefined
		documentationLink?: string | undefined
		eeOnly?: boolean
		small?: boolean
		wrapperClass?: string
		headerClass?: string
		collapsable?: boolean
		collapsed?: boolean
		headless?: boolean
		animate?: boolean
		breakAll?: boolean
		class?: string | undefined
		header?: import('svelte').Snippet
		action?: import('svelte').Snippet
		badge?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		label = undefined,
		tooltip = undefined,
		documentationLink = undefined,
		eeOnly = false,
		small = false,
		wrapperClass = '',
		headerClass = '',
		collapsable = false,
		collapsed = $bindable(true),
		headless = false,
		animate = false,
		breakAll = false,
		class: clazz = undefined,
		header,
		action,
		badge,
		children
	}: Props = $props()
</script>

<div class={twMerge('w-full flex flex-col', wrapperClass)}>
	{#if !headless}
		<div class="flex flex-row justify-between items-center mb-2">
			<h2
				class={twMerge(
					'font-semibold flex flex-row items-center gap-1',
					breakAll ? 'break-all' : '',
					small ? 'text-sm' : 'text-base',
					headerClass
				)}
			>
				{#if collapsable}
					<button class="flex items-center gap-1" onclick={() => (collapsed = !collapsed)}>
						<ChevronRight
							size={16}
							class={twMerge(
								'transition',
								collapsed ? '' : 'rotate-90',
								animate ? 'duration-200' : 'duration-0'
							)}
						/>
						{label}
					</button>
				{:else}
					{label}
				{/if}

				{@render header?.()}
				{#if tooltip}
					<Tooltip {documentationLink}>{tooltip}</Tooltip>
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
			{@render action?.()}
			{#if collapsable && collapsed}
				{@render badge?.()}
			{/if}
		</div>
	{/if}
	{#if !collapsable || !collapsed}
		<div
			class={twMerge('grow min-h-0', clazz)}
			transition:slide={animate ? { duration: 200 } : { duration: 0 }}
		>
			{@render children?.()}
		</div>
	{/if}
</div>
