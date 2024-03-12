<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import Required from './Required.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { capitalize, emptyString } from '$lib/utils'

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
</script>

<div class="inline-flex flex-row items-baseline truncated">
	<span class={twMerge(disabled ? 'text-tertiary' : '', 'font-semibold', labelClass)}>
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
		{#if format && !format.startsWith('resource')}
			<span class="text-xs italic ml-2 text-tertiary dark:text-indigo-400">
				{format}
			</span>
		{:else}
			<span class="text-xs italic ml-2 text-tertiary dark:text-indigo-400">
				{type ?? 'any'}{contentEncoding && contentEncoding != ''
					? `, encoding: ${contentEncoding}`
					: ''}
			</span>
		{/if}
	{/if}

	{#if !emptyString(simpleTooltip)}
		<Tooltip class="ml-2">
			<span class="text-xs">
				{simpleTooltip}
			</span>
		</Tooltip>
	{/if}
</div>
