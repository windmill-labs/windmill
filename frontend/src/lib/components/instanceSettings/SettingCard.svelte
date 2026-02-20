<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { enterpriseLicense } from '$lib/stores'
	import EEOnly from '../EEOnly.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { Button } from '../common'
	import type { ButtonType } from '../common/button/model'

	interface Props {
		label?: string
		description?: string
		ee_only?: string
		tooltip?: string
		settingKey?: string
		actionButton?: {
			label: string
			onclick: (values: Record<string, any>) => Promise<void>
			variant?: ButtonType.Variant
		}
		values?: Record<string, any>
		children: import('svelte').Snippet
		class?: string
	}

	let {
		label,
		description,
		ee_only,
		tooltip,
		settingKey,
		actionButton,
		values,
		children,
		class: clazz
	}: Props = $props()
</script>

<div
	data-setting-key={settingKey}
	class={twMerge('p-4 rounded-md bg-surface-tertiary shadow-sm flex flex-col gap-1', clazz)}
>
	{#if label}
		<div class="flex items-center justify-between gap-2 w-full">
			<div class="flex gap-1 items-baseline">
				<span class="text-emphasis font-semibold text-xs">{label}</span>
				{#if ee_only != undefined && !$enterpriseLicense}
					{#if ee_only != ''}
						<EEOnly>{ee_only}</EEOnly>
					{:else}
						<EEOnly />
					{/if}
				{/if}
				{#if tooltip}
					<Tooltip>{tooltip}</Tooltip>
				{/if}
			</div>
			{#if actionButton}
				<Button
					disabled={ee_only != undefined && !$enterpriseLicense}
					variant={actionButton.variant ?? 'default'}
					unifiedSize="sm"
					onclick={async () => await actionButton?.onclick(values ?? {})}
				>
					{actionButton.label}
				</Button>
			{/if}
		</div>
		{#if description}
			<span class="text-secondary font-normal text-xs">
				{@html description}
			</span>
		{/if}
	{/if}

	{@render children()}
</div>
