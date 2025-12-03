<script lang="ts">
	interface Props {
		completed: number
		total: number
		label?: string
	}

	let { completed, total, label = 'tutorials' }: Props = $props()

	const progressPercentage = $derived(
		total > 0 ? Math.round((completed / total) * 100) : 0
	)
</script>

<div class="mb-6">
	<div class="flex items-center justify-between mb-2">
		<div class="text-xs font-semibold text-emphasis">
			Progress: {completed} of {total} {label} completed
		</div>
		<div class="text-xs font-normal text-secondary">{progressPercentage}%</div>
	</div>
	<div class="w-full h-2 bg-surface-secondary rounded-full overflow-hidden">
		<div
			class="h-full bg-surface-accent-primary transition-all duration-300 ease-out rounded-full"
			style="width: {progressPercentage}%"
		></div>
	</div>
</div>

