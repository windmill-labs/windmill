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
	class="group relative flex items-center gap-4 w-full px-4 py-3 rounded-lg border transition-all duration-200 text-left border-light bg-surface hover:bg-surface-hover hover:border-normal"
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
	<div class="flex items-center gap-1.5 flex-shrink-0">
		{#if isCompleted}
			<CheckCircle2 size={14} class="text-green-500 flex-shrink-0" />
		{/if}
		<span
			class="text-xs font-medium {isCompleted
				? 'text-green-500'
				: 'text-secondary'}"
		>
			{isCompleted ? 'Completed' : 'Not started'}
		</span>
	</div>
</button>

