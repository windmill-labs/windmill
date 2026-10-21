<script lang="ts">
	import { ArrowUp, ChevronDown, MessagesSquare } from 'lucide-svelte'
</script>

<!-- The AI composer at rest, one line, with HomeAIChat's own margins around it. -->
<div class="mt-2 w-full pb-2">
	<div
		class="mt-1 flex w-full items-center gap-3 rounded-[10px] border border-border-light bg-surface-input px-4 py-3"
	>
		<MessagesSquare size={16} class="shrink-0 text-ai" />
		<span class="min-w-0 flex-1 truncate text-sm text-hint">
			Ask about this workspace, or describe what to build…
		</span>
		<span class="hidden shrink-0 items-center gap-1 text-xs text-secondary sm:inline-flex">
			claude-sonnet-4-6 · high
			<ChevronDown size={12} />
		</span>
		<ArrowUp size={14} class="shrink-0 text-hint" />
	</div>
</div>
