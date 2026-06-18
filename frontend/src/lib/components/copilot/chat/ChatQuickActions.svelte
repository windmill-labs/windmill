<script lang="ts">
	import { SparklesIcon, LightbulbIcon, DiffIcon } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	let iconClasses = '!w-3 !h-3 !px-0 !m-0'

	let {
		diffMode,
		askAi
	}: {
		diffMode: boolean
		askAi: (instructions: string, options?: { withCode?: boolean; withDiff?: boolean }) => void
	} = $props()

	let btnClasses = $derived(
		`!px-1 !py-0.5 !gap-1 ${
			diffMode
				? '!bg-surface dark:!bg-surface border-frost-500 dark:border-frost-500'
				: '!font-normal'
		}`
	)
</script>

<div class="flex flex-row items-center gap-2 pr-1 py-1">
	<div class="flex flex-row items-center gap-1.5">
		{#if diffMode}
			<Button
				on:click={() => {
					askAi(
						'Based on the changes I made to the code, look for potential issues and recommend better solutions',
						{ withDiff: true }
					)
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
					askAi('Explain the changes I made to the code from the last diff', {
						withCode: false,
						withDiff: true
					})
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
					askAi('Look for potential issues and recommend better solutions in the actual code')
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
