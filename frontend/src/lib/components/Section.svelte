<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import { ChevronRight } from 'lucide-svelte'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'
	import { slide } from 'svelte/transition'
	import EEOnly from './EEOnly.svelte'

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
		description?: string | undefined
		initiallyCollapsed?: boolean
		header?: import('svelte').Snippet
		action?: import('svelte').Snippet
		badge?: import('svelte').Snippet
		children?: import('svelte').Snippet
		labelExtra?: import('svelte').Snippet
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
		initiallyCollapsed = true,
		collapsed = $bindable(initiallyCollapsed),
		headless = false,
		animate = false,
		breakAll = false,
		class: clazz = undefined,
		description = undefined,
		header,
		action,
		badge,
		children,
		labelExtra
	}: Props = $props()
</script>

<div class={twMerge('w-full flex flex-col', wrapperClass)}>
	{#if !headless}
		<div class="flex flex-row justify-between items-center">
			<h2
				class={twMerge(
					'text-emphasis flex flex-row items-center gap-1',
					breakAll ? 'break-all' : '',
					small ? 'text-xs font-semibold' : 'text-sm font-semibold',
					headerClass
				)}
			>
				{#if collapsable}
					<button class="flex items-center gap-1" onclick={() => (collapsed = !collapsed)}>
						{label}
						{@render labelExtra?.()}
						<ChevronRight
							size={14}
							class={twMerge('transition duration-200', collapsed ? '' : 'rotate-90')}
						/>
					</button>
				{:else}
					{label}
					{@render labelExtra?.()}
				{/if}

				{@render header?.()}
				{#if tooltip}
					<Tooltip {documentationLink}>{tooltip}</Tooltip>
				{/if}
				{#if eeOnly}
					{#if !$enterpriseLicense}
						<EEOnly />
					{/if}
				{/if}
			</h2>
			{#if !(collapsable && collapsed)}
				{@render action?.()}
			{/if}
			{#if collapsable && collapsed}
				{@render badge?.()}
			{/if}
		</div>
	{/if}
	{#if !collapsable || !collapsed}
		<div
			class={'grow min-h-0 '}
			transition:slide={animate || collapsable ? { duration: 200 } : { duration: 0 }}
		>
			{#if description}
				<div class="text-xs text-primary mt-1 mb-2">{@html description}</div>
			{/if}
			<div class="flex flex-col gap-6 grow min-h-0 mt-4">
				<div class={twMerge('grow min-h-0', clazz)}>
					{@render children?.()}
				</div>
			</div>
		</div>
	{/if}
</div>
