<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'
	import { capitalize, emptyString } from '$lib/utils'
	import Tooltip from './meltComponents/Tooltip.svelte'
	import { InfoIcon } from 'lucide-svelte'

	export let label: string
	export let format: string = ''
	export let contentEncoding = ''
	export let type: string | undefined = undefined
	export let disabled: boolean = false
	export let required = false
	export let displayType: boolean = true
	export let labelClass: string = ''
	export let prettify = false
	export let simpleTooltip: string | undefined = undefined
	export let lightHeader = false
	export let SimpleTooltipIcon = InfoIcon
	export let simpleTooltipIconClass = ''
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
			<span class="text-xs" slot="text">
				{simpleTooltip}
			</span>
		</Tooltip>
	{/if}
</div>
