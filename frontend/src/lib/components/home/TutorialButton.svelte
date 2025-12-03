<script lang="ts">
	import { CheckCircle2 } from 'lucide-svelte'
	import type { ComponentType } from 'svelte'

	interface Props {
		icon: ComponentType
		title: string
		description: string
		onclick: () => void
		isCompleted?: boolean
	}

	let { icon: Icon, title, description, onclick, isCompleted = false }: Props = $props()
</script>

<button
	onclick={onclick}
	class="group relative flex items-center gap-4 w-full px-4 py-3 rounded-lg border transition-all duration-200 text-left hover:shadow-md hover:-translate-y-0.5 border-border/50 bg-surface hover:bg-surface-hover hover:border-primary/30"
>
	<!-- Icon -->
	<div
		class="flex-shrink-0 p-2.5 rounded-lg transition-colors bg-primary/10 dark:bg-primary/20 group-hover:bg-primary/15"
	>
		<Icon size={20} class="transition-colors text-primary" />
	</div>

	<!-- Content -->
	<div class="flex-1 min-w-0">
		<div class="flex items-center gap-2 mb-1">
			<h3 class="text-sm font-semibold text-emphasis group-hover:text-primary transition-colors">
				{title}
			</h3>
		</div>
		<p class="text-xs text-secondary line-clamp-2">{description}</p>
	</div>

	<!-- Status -->
	<span
		class="text-xs font-medium flex-shrink-0 {isCompleted
			? 'text-green-600'
			: 'text-primary'}"
	>
		{isCompleted ? 'Completed' : 'Not started'}
	</span>
	{#if isCompleted}
				<CheckCircle2 size={14} class="text-green-600 flex-shrink-0" />
	{/if}
</button>

