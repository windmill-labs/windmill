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
	import type { ComponentType } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	interface Props {
		label: string
		selected?: boolean
		returnIcon?: boolean
		onSelect: () => void
	}

	let { label, selected, returnIcon, onSelect }: Props = $props()

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
		'AI Agent': { icon: BotIcon, iconClass: 'text-ai' },
		MCP: { icon: Plug, showChevron: true }
	}

	const config = $derived(iconMap[label])
</script>

{#snippet iconWithText(icon: ComponentType, showChevron = false, iconClass = '')}
	{label}
	{#if showChevron}
		<ChevronRight size={12} class="ml-auto text-secondary" />
	{/if}
{/snippet}

<Button
	id={`flow-editor-flow-kind-${label.replaceAll(' ', '-').toLowerCase()}`}
	{selected}
	onClick={onSelect}
	variant="subtle"
	unifiedSize="sm"
	startIcon={{ icon: config?.icon }}
>
	<span class="grow flex items-center gap-2">
		{#if config}
			{@render iconWithText(config.icon, config.showChevron, config.iconClass ?? '')}
		{/if}
	</span>
	{#if returnIcon && selected}
		<kbd class="!text-xs text-right">&crarr;</kbd>
	{/if}
</Button>
