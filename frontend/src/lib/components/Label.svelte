<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'
	import Tooltip from './Tooltip.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import EEOnly from './EEOnly.svelte'

	interface Props {
		label?: string | undefined
		disabled?: boolean
		headless?: boolean
		required?: boolean
		headerClass?: string
		class?: string | undefined
		for?: string | undefined
		tooltip?: string | undefined
		eeOnly?: boolean
		header?: import('svelte').Snippet
		error?: import('svelte').Snippet
		action?: import('svelte').Snippet
		children?: import('svelte').Snippet
	}

	let {
		label = undefined,
		disabled = false,
		headless = false,
		required = false,
		headerClass = '',
		class: clazz = undefined,
		for: forAttr = undefined,
		tooltip = undefined,
		eeOnly = false,
		header,
		error,
		action,
		children
	}: Props = $props()
</script>

<div
	class={twMerge('flex flex-col gap-1', disabled ? 'opacity-60 pointer-events-none' : '', clazz)}
>
	<div class="flex flex-row justify-between items-center w-full">
		{#if !headless}
			<div class={twMerge('flex flex-row items-center gap-2', headerClass)}>
				<span class="text-emphasis text-xs font-semibold whitespace-nowrap"
					><label for={forAttr}>{label}</label>
					{#if required}
						<Required required={true} />
					{/if}
					{#if tooltip}
						<Tooltip>{tooltip}</Tooltip>
					{/if}
				</span>
				{#if eeOnly}
					{#if !$enterpriseLicense}
						<EEOnly />
					{/if}
				{/if}
				{@render header?.()}
			</div>
		{/if}
		{@render error?.()}
		{@render action?.()}
	</div>
	{@render children?.()}
</div>
