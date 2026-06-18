<script lang="ts">
	import type { ComponentType } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import EEOnly from '$lib/components/EEOnly.svelte'
	import { enterpriseLicense } from '$lib/stores'

	interface NavigationItem {
		id: string
		label: string
		icon?: ComponentType
		disabled?: boolean
		count?: number
		aiId?: string
		aiDescription?: string
		showIf?: boolean
		isEE?: boolean
	}

	interface NavigationGroup {
		title?: string
		items: NavigationItem[]
	}

	interface Props {
		groups: NavigationGroup[]
		selectedId: string
		onNavigate: (id: string) => void
		class?: string
	}

	let { groups, selectedId, onNavigate, class: className = '' }: Props = $props()
</script>

<div class={twMerge('flex flex-col gap-6', className)}>
	{#each groups as group (group.title)}
		<div class="flex flex-col gap-1">
			{#if group.title}
				<div class="text-sm font-semibold text-emphasis px-2 mb-1">
					{group.title}
				</div>
			{/if}
			<nav class="flex flex-col gap-0.5">
				{#each group.items as item (item.id)}
					{#if item.showIf !== false}
						{@const isSelected = selectedId === item.id}
						<Button
							variant="subtle"
							unifiedSize="sm"
							selected={isSelected}
							disabled={item.disabled}
							aiId={item.aiId}
							aiDescription={item.aiDescription}
							startIcon={item.icon ? { icon: item.icon } : undefined}
							btnClasses={'!justify-start text-left !w-full'}
							onClick={() => onNavigate(item.id)}
						>
							<span class="truncate">{item.label}</span>
							<div class="ml-auto flex items-center gap-1">
								{#if item.isEE && !$enterpriseLicense}
									<EEOnly />
								{/if}
								{#if item.count !== undefined}
									<span
										class="text-2xs text-secondary bg-surface-secondary px-1.5 py-0.5 rounded-full"
									>
										{item.count}
									</span>
								{/if}
							</div>
						</Button>
					{/if}
				{/each}
			</nav>
		</div>
	{/each}
</div>
