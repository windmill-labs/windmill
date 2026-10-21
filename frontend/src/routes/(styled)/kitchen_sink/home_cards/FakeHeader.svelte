<script lang="ts">
	import { Building2, ChevronDown, ExternalLink, Globe2, PanelLeft, PlugZap } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	interface Props {
		name: string
		drafts: number
	}

	let { name, drafts }: Props = $props()
</script>

<!-- Home's header band: the workspace crumb, the drafts it holds and the side trips. -->
<div class="flex h-12 min-w-0 shrink-0 items-center gap-1 pl-2 pr-4">
	<span class="flex h-8 w-8 items-center justify-center text-secondary md:hidden">
		<PanelLeft size={16} />
	</span>
	<span class="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-semibold text-emphasis">
		<Building2 size={14} class="text-secondary" />
		{name}
		<ChevronDown size={14} class="text-secondary" />
	</span>
	<span class="flex-1"></span>
	{#if drafts > 0}
		<span
			class="rounded-md bg-blue-50 px-2 py-0.5 text-2xs text-blue-800 dark:bg-blue-700/40 dark:text-blue-100"
		>
			{drafts} drafts
		</span>
		<span class="hidden sm:inline">
			<Button variant="subtle" unifiedSize="sm">Review & deploy drafts</Button>
		</span>
	{/if}
	<span class="hidden items-center gap-1 lg:flex">
		<Button
			variant="subtle"
			unifiedSize="xs"
			btnClasses="!text-2xs !text-hint"
			startIcon={{ icon: PlugZap }}
		>
			CLI / MCP
		</Button>
		<Button
			variant="subtle"
			unifiedSize="xs"
			btnClasses="!text-2xs !text-hint"
			startIcon={{ icon: Globe2 }}
			endIcon={{ icon: ExternalLink }}
		>
			Hub
		</Button>
	</span>
</div>
