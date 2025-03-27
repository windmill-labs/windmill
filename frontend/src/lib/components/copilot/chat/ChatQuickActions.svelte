<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { SparklesIcon, LightbulbIcon } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'

	export let hasDiff: boolean
	export let diffMode: boolean = false

	const dispatch = createEventDispatcher<{
		askAiAboutChanges: null
		suggestImprovements: null
	}>()
</script>

<div class="flex flex-row items-center gap-2 px-2 py-1">
	<div class="flex flex-row items-center gap-1.5">
		<Button
			on:click={() => {
				dispatch('askAiAboutChanges')
			}}
			title="Explain changes"
			size="xs"
			btnClasses="!px-2 !py-0.5 {diffMode ? '!bg-surface-secondary dark:!bg-surface-secondary' : ''}"
			startIcon={{ icon: SparklesIcon }}
			variant="border"
			color="light"
			propagateEvent
			disabled={!hasDiff}
		>
			Explain
		</Button>
		<Button
			on:click={() => {
				dispatch('suggestImprovements')
			}}
			title="Suggest improvements"
			size="xs"
			btnClasses="!px-2 !py-0.5 {diffMode ? '!bg-surface-secondary dark:!bg-surface-secondary' : ''}"
			startIcon={{ icon: LightbulbIcon }}
			variant="border"
			color="light"
			propagateEvent
			disabled={!hasDiff}
		>
			Improve
		</Button>
	</div>
</div> 