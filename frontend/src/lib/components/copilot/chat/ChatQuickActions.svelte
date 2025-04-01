<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { SparklesIcon, LightbulbIcon } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let hasDiff: boolean
	export let diffMode: boolean = false

	let iconClasses = '!w-3 !h-3 !px-0 !m-0'

	const dispatch = createEventDispatcher<{
		analyzeChanges: null
		explainChanges: null
	}>()

	$: btnClasses = `!px-1 !py-0.5 !gap-1 ${
		diffMode ? '!bg-surface-secondary dark:!bg-surface-secondary' : '!font-normal'
	}`
</script>

<div class="flex flex-row items-center gap-2 px-2 py-1">
	<div class="flex flex-row items-center gap-1.5">
		{#if !diffMode}
			<Button
				on:click={() => {
					dispatch('explainChanges')
				}}
				title="Explain changes"
				size="xs3"
				{btnClasses}
				startIcon={{ icon: LightbulbIcon, classes: iconClasses }}
				variant="border"
				color="light"
				propagateEvent
				disabled={!hasDiff}
			>
				Explain
			</Button>
		{/if}
		<Button
			on:click={() => {
				dispatch('analyzeChanges')
			}}
			title="Suggest improvements"
			size="xs"
			{btnClasses}
			startIcon={{ icon: diffMode ? SparklesIcon : LightbulbIcon, classes: iconClasses }}
			variant="border"
			color="light"
			propagateEvent
			disabled={!hasDiff}
		>
			{diffMode ? 'Analyze' : 'Improve'}
		</Button>
	</div>
</div>
