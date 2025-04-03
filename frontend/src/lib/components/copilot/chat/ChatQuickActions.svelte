<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { SparklesIcon, LightbulbIcon, DiffIcon } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let diffMode: boolean = false

	let iconClasses = '!w-3 !h-3 !px-0 !m-0'

	const dispatch = createEventDispatcher<{
		analyzeChanges: null
		explainChanges: null
		suggestImprovements: null
	}>()

	$: btnClasses = `!px-1 !py-0.5 !gap-1 ${
		diffMode
			? '!bg-surface dark:!bg-surface border-frost-500 dark:border-frost-500'
			: '!font-normal'
	}`
</script>

<div class="flex flex-row items-center gap-2 px-2 py-1">
	<div class="flex flex-row items-center gap-1.5">
		{#if diffMode}
			<Button
				on:click={() => {
					dispatch('analyzeChanges')
				}}
				title="Analyze changes"
				size="xs"
				{btnClasses}
				startIcon={{ icon: SparklesIcon, classes: iconClasses }}
				variant="border"
				color="light"
				propagateEvent
			>
				Analyze
			</Button>
		{:else}
			<Button
				on:click={() => {
					dispatch('explainChanges')
				}}
				title="Explain changes"
				size="xs3"
				{btnClasses}
				startIcon={{ icon: DiffIcon, classes: iconClasses }}
				variant="border"
				color="light"
				propagateEvent
			>
				Explain
			</Button>
			<Button
				on:click={() => {
					dispatch('suggestImprovements')
				}}
				title="Suggest improvements"
				size="xs"
				{btnClasses}
				startIcon={{ icon: LightbulbIcon, classes: iconClasses }}
				variant="border"
				color="light"
				propagateEvent
			>
				Improve
			</Button>
		{/if}
	</div>
</div>
