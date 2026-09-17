<script lang="ts">
	import type { FlowModule } from '$lib/gen'
	import { getContext } from 'svelte'
	import { ChevronRight } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import type { FlowEditorContext } from '../types'

	interface Props {
		/** The agents a tool sits under, the step's own first. */
		agents: Pick<FlowModule, 'id' | 'summary'>[]
	}

	let { agents }: Props = $props()

	const { selectionManager } = getContext<FlowEditorContext>('FlowEditorContext')
</script>

<nav
	aria-label="Breadcrumb"
	class="flex flex-row flex-wrap items-center gap-0.5 min-w-0 text-xs text-secondary"
>
	{#each agents as agent, i (i)}
		<Button
			variant="subtle"
			unifiedSize="2xs"
			onClick={() => selectionManager.selectId(agent.id, { openPanel: true })}
			wrapperClasses="min-w-0 shrink"
			btnClasses="!px-0 !font-normal !text-xs text-secondary hover:text-emphasis hover:underline hover:!bg-transparent min-w-0"
		>
			<span class="truncate">{agent.summary || 'AI Agent'}</span>
		</Button>
		<ChevronRight size={12} class="text-tertiary shrink-0" />
	{/each}
</nav>
