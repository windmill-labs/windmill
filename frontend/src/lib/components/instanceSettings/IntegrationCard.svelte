<script lang="ts">
	import { Button } from '$lib/components/common'
	import { ChevronDown, Plus, Settings } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		title: string
		icon: any
		children: any
		hasChannels: boolean
		expandable?: boolean
		expanded?: boolean
		forceExpanded?: boolean
		onAdd?: () => void
		onToggleExpand?: () => void
		isPlaceholder?: boolean
		actions?: import('svelte').Snippet
		class?: string
		style?: string
	}

	let {
		title,
		icon: Icon,
		children,
		hasChannels,
		expandable = false,
		expanded = false,
		forceExpanded = false,
		onAdd,
		onToggleExpand,
		isPlaceholder = false,
		actions,
		class: clazz,
		style
	}: Props = $props()
</script>

{#if isPlaceholder}
	<!-- Placeholder Card (dashed border) -->
	<div
		class={twMerge(
			'border border-dashed border-gray-600 flex h-[67px] items-center justify-center px-4 py-4 rounded-md w-full bg-transparent hover:bg-surface-secondary/50 transition-colors cursor-pointer',
			clazz
		)}
		{style}
		onclick={onAdd}
		role="button"
		tabindex="0"
		onkeydown={(e) => e.key === 'Enter' && onAdd?.()}
	>
		<div class="flex gap-2 items-center justify-center">
			<Plus size={14} class="text-secondary" />
			<span class="text-xs font-medium text-secondary text-center">
				Add {title.toLowerCase()} channel
			</span>
			<Icon size={title === 'Microsoft Teams' ? 20 : 14} class="text-secondary" />
		</div>
	</div>
{:else}
	<!-- Connected Card (solid background) -->
	<div
		class={twMerge('bg-surface-tertiary border flex flex-col gap-2 p-4 rounded-md w-full', clazz)}
		{style}
	>
		<!-- Card Header -->
		<div class="flex items-center justify-between w-full">
			<div class="flex gap-2 items-center">
				<Icon size={20} class="text-primary" />
				<span class="text-xs font-semibold text-primary">{title}</span>
			</div>

			{@render actions?.()}

			<div class="flex items-center gap-2">
				{#if !forceExpanded && expandable && hasChannels && onToggleExpand}
					<Button
						variant="default"
						unifiedSize="sm"
						onclick={onToggleExpand}
						aria-label="Toggle {expanded ? 'collapse' : 'expand'}"
						startIcon={{ icon: Settings }}
					>
						<div class="flex items-center justify-center gap-2">
							<span>Configure SMTP</span>
							<ChevronDown
								size={14}
								class={twMerge('transition-transform', expanded ? 'transform rotate-180' : '')}
							/>
						</div>
					</Button>
				{/if}
			</div>
		</div>

		<!-- Card Content -->
		<div class="space-y-2">
			{@render children()}
		</div>
	</div>
{/if}
