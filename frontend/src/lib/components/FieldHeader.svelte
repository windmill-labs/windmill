<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'
	import { capitalize, emptyString } from '$lib/utils'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { InfoIcon } from 'lucide-svelte'

	interface Props {
		label: string;
		format?: string;
		contentEncoding?: string;
		type?: string | undefined;
		disabled?: boolean;
		required?: boolean;
		displayType?: boolean;
		labelClass?: string;
		prettify?: boolean;
		simpleTooltip?: string | undefined;
		lightHeader?: boolean;
		SimpleTooltipIcon?: any;
		simpleTooltipIconClass?: string;
	}

	let {
		label,
		format = '',
		contentEncoding = '',
		type = undefined,
		disabled = false,
		required = false,
		displayType = true,
		labelClass = '',
		prettify = false,
		simpleTooltip = undefined,
		lightHeader = false,
		SimpleTooltipIcon = InfoIcon,
		simpleTooltipIconClass = ''
	}: Props = $props();
</script>

<div class="inline-flex flex-row items-baseline truncated">
	<span
		class={twMerge(
			disabled ? 'text-primary' : '',
			'font-semibold text-xs',
			lightHeader ? 'text-secondary font-normal' : 'text-emphasis',
			labelClass
		)}
	>
		{#if prettify}
			{label.replace(/_/g, ' ').split(' ').map(capitalize).join(' ')}
		{:else}
			{label}
		{/if}
	</span>
	{#if !disabled}
		<Required {required} class="!ml-0" />
	{/if}

	{#if displayType}
		<span class="text-2xs italic ml-2 text-hint">
			{#if format && !format.startsWith('resource') && !format.startsWith('jsonschema-')}
				{format}
			{:else}
				{type ?? 'any'}{contentEncoding && contentEncoding != ''
					? `, encoding: ${contentEncoding}`
					: ''}
			{/if}
		</span>
	{/if}

	{#if !emptyString(simpleTooltip)}
		<Tooltip class="ml-2" placement="bottom">
			<SimpleTooltipIcon size="14" class={'-mb-0.5 ' + simpleTooltipIconClass} />
			{#snippet text()}
						<span class="text-xs" >
					{simpleTooltip}
				</span>
					{/snippet}
		</Tooltip>
	{/if}
</div>
