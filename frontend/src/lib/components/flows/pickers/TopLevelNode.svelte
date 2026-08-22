<script lang="ts">
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import {
		BotIcon,
		CheckCircle2,
		ChevronRight,
		Code,
		GitBranch,
		Globe,
		Plug,
		Repeat,
		Square,
		Zap
	} from 'lucide-svelte'
	import type { ComponentType } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	interface Props {
		label: string
		selected?: boolean
		returnIcon?: boolean
		/** Highlight with the neutral hover surface instead of the accent, for transient
		 * (hover/keyboard) selection rather than the persistent category selection. */
		neutral?: boolean
		onSelect: () => void
		onHover?: () => void
	}

	let { label, selected, returnIcon, neutral = false, onSelect, onHover }: Props = $props()

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
		'AI Agent': { icon: BotIcon, showChevron: true, iconClass: 'text-ai' },
		'AI Sandbox': { icon: BotIcon, showChevron: true, iconClass: 'text-ai' },
		'Claude Code': { icon: BotIcon, iconClass: 'text-ai' },
		MCP: { icon: Plug },
		'Web Search': { icon: Globe }
	}

	const config = $derived(iconMap[label])
</script>

{#snippet iconWithText(icon: ComponentType, showChevron = false, iconClass = '')}
	<span class="truncate">{label}</span>
	{#if showChevron}
		<ChevronRight size={12} class="ml-auto shrink-0 text-secondary" />
	{/if}
{/snippet}

<Button
	id={`flow-editor-flow-kind-${label.replaceAll(' ', '-').toLowerCase()}`}
	selected={neutral ? false : selected}
	onClick={onSelect}
	onmousemove={() => onHover?.()}
	variant="subtle"
	unifiedSize="sm"
	startIcon={{ icon: config?.icon }}
	btnClasses={neutral ? (selected ? 'bg-surface-hover' : 'hover:bg-transparent') : ''}
>
	<span class="grow min-w-0 flex items-center gap-2">
		{#if config}
			{@render iconWithText(config.icon, config.showChevron, config.iconClass ?? '')}
		{/if}
	</span>
	{#if returnIcon && selected}
		<kbd class="!text-xs text-right">&crarr;</kbd>
	{/if}
</Button>
