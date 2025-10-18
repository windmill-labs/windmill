<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import {
		BotIcon,
		CheckCircle2,
		ChevronRight,
		Code,
		GitBranch,
		Plug,
		Repeat,
		Square,
		Zap
	} from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ComponentType } from 'svelte'

	interface Props {
		label: string
		selected?: boolean
		returnIcon?: boolean
		onSelect: () => void
		class?: string
	}

	let { label, selected, returnIcon, onSelect, class: className }: Props = $props()

	interface IconConfig {
		icon: ComponentType
		showChevron?: boolean
		iconClass?: string
	}

	const iconMap: Record<string, IconConfig> = {
		Action: { icon: Code, showChevron: true },
		Trigger: { icon: Zap, showChevron: true },
		'Approval/Prompt': { icon: CheckCircle2, showChevron: true },
		Flow: { icon: BarsStaggered as unknown as ComponentType, showChevron: true },
		'End Flow': { icon: Square },
		'For loop': { icon: Repeat },
		'While loop': { icon: Repeat },
		'Branch to one': { icon: GitBranch },
		'Branch to all': { icon: GitBranch },
		'AI Agent': { icon: BotIcon, iconClass: 'text-violet-800 dark:text-violet-400' },
		MCP: { icon: Plug, showChevron: true }
	}

	const config = $derived(iconMap[label])
</script>

{#snippet iconWithText(icon: ComponentType, showChevron = false, iconClass = '')}
	{@const Icon = icon}
	<Icon size={14} class={iconClass} />
	{label}
	{#if showChevron}
		<ChevronRight size={12} class="ml-auto" color="#4c566a" />
	{/if}
{/snippet}

<button
	id={`flow-editor-flow-kind-${label.replaceAll(' ', '-').toLowerCase()}`}
	class={twMerge(
		'w-full text-left py-2 px-1.5 hover:bg-surface-hover text-xs font-medium transition-all whitespace-nowrap flex flex-row gap-2 items-center rounded-md',
		selected ? 'bg-surface-hover' : '',
		className
	)}
	onpointerdown={onSelect}
	role="menuitem"
	tabindex="-1"
>
	<span class="grow flex items-center gap-2">
		{#if config}
			{@render iconWithText(config.icon, config.showChevron, config.iconClass ?? '')}
		{/if}
	</span>
	{#if returnIcon && selected}
		<kbd class="!text-xs text-right">&crarr;</kbd>
	{/if}
</button>
