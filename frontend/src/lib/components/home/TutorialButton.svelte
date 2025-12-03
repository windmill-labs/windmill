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
	class="group relative flex items-center gap-4 w-full px-4 py-3 first-of-type:!border-t-0 first-of-type:rounded-t-md last-of-type:rounded-b-md [*:not(:last-child)]:border-b border-b border-light hover:bg-surface-hover transition-colors text-left last:border-b-0"
>
	<!-- Icon -->
	<Icon size={20} class="flex-shrink-0 text-accent-primary transition-colors" />

	<!-- Content -->
	<div class="flex-1 min-w-0">
		<div class="text-emphasis flex-wrap text-left text-xs font-semibold group-hover:text-accent-primary transition-colors">
			{title}
		</div>
		<div class="text-hint text-3xs truncate text-left font-normal">
			{description}
		</div>
	</div>

	<!-- Status -->
	<div class="flex items-center gap-1.5 flex-shrink-0">
		{#if isCompleted}
			<CheckCircle2 size={14} class="text-green-500 flex-shrink-0" />
		{/if}
		<span
			class="text-xs font-normal {isCompleted
				? 'text-green-500'
				: 'text-secondary'}"
		>
			{isCompleted ? 'Completed' : 'Not started'}
		</span>
	</div>
</button>

