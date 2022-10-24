<script lang="ts">
	import ProgressBarPart from './ProgressBarPart.svelte'

	export let i = -1
	export let steps: number = 0

	$: series = Array.from(Array(steps).keys()).map((x) => Math.floor(((100 / steps) * x) / 100))

	function toggle() {
		toggled[i] = true
	}

	$: toggled = series.map(() => false)
	$: text = i < series.length ? `Step ${i + 1}` : 'Done'
	$: toggled[i] === false && toggle()
</script>

<div>
	<div class="flex justify-between mb-1">
		<span class="text-base font-medium text-blue-700 dark:text-white">{text}</span>
		<span class="text-sm font-medium text-blue-700 dark:text-white">
			{series.slice(0, i).reduce((x, y) => y + x, 0)}%
		</span>
	</div>
	<div class="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700 relative">
		{#each series as serie, index}
			<ProgressBarPart
				isFirst={index === 0}
				isLast={index === series.length - 1}
				sumUpTo={series.slice(0, index).reduce((x, y) => y + x, 0)}
				length={serie}
				shouldToggle={toggled[index]}
			/>
		{/each}
	</div>
</div>
