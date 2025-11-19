<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { FlowNodeColorClasses } from '$lib/components/graph'
	import { Pencil } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'

	let iconWidth: number = $state(0)
	let idBadgeWidth: number | undefined = $state(undefined)
	interface Props {
		label?: string
		path?: string
		id?: string
		deletable?: boolean
		bold?: boolean
		editId?: boolean
		hover?: boolean
		colorClasses?: FlowNodeColorClasses
		icon?: import('svelte').Snippet
		onclick?: () => void
	}

	let {
		label = '',
		path = '',
		id = '',
		deletable = false,
		bold = false,
		editId = $bindable(false),
		hover = false,
		colorClasses,
		icon,
		onclick
	}: Props = $props()

	let marginLeft = $derived(Math.max(iconWidth ?? 0, idBadgeWidth ?? 0) * 2 + 32)
</script>

<div
	id={id || undefined}
	class="relative flex gap-1 justify-between items-center w-full overflow-hidden rounded-sm
	 p-2 text-2xs module text-primary"
>
	{#if icon && true}
		<div class="flex-none" bind:clientWidth={iconWidth}>
			{@render icon?.()}
		</div>
	{/if}

	<Popover
		class="absolute left-1/2 transform -translate-x-1/2 center-center"
		style="max-width: calc(100% - {marginLeft}px)"
	>
		<div class="text-center {colorClasses?.text} truncate {bold ? '!font-bold' : 'font-normal'}">
			{label}
		</div>
		{#snippet text()}
			<div>
				<div>{label}</div>
				{#if path != ''}<div>{path}</div>{/if}
			</div>
		{/snippet}
	</Popover>

	<div class="flex items-center space-x-2 relative" bind:clientWidth={idBadgeWidth}>
		{#if id && id !== 'preprocessor' && !id.startsWith('failure') && !id.startsWith('subflow:')}
			<Badge
				color="transparent"
				class="border-none"
				wrapperClass={twMerge(
					'max-w-full rounded-md hover:opacity-60 transition-opacity',
					colorClasses?.badge
				)}
				baseClass={twMerge('!px-1')}
				title={id}
				clickable
				onclick={(e) => {
					e?.preventDefault()
					e?.stopPropagation()
					editId = !editId
					onclick?.()
				}}
			>
				<span class="max-w-full text-2xs truncate flex items-center">
					{#if editId || (hover && deletable)}
						<span transition:slide={{ axis: 'x', duration: 100 }}>
							<Pencil size={10} class="mr-1" />
						</span>
					{/if}
					<span class="max-w-12 truncate">
						{id}
					</span>
				</span>
			</Badge>
		{:else if id?.startsWith('subflow:')}
			<Badge color="blue" wrapperClass="max-w-full" baseClass="!px-1" title={id}>
				<span class="max-w-full text-2xs truncate">{id.substring('subflow:'.length)}</span></Badge
			>
		{/if}
	</div>
</div>
