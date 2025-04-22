<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'
	import Description from '$lib/components/Description.svelte'
	import { Alert } from '$lib/components/common'

	let scheduleEditor = $state<ScheduleEditor | null>(null)
	let { selectedTrigger, isFlow, path, isDeployed = false } = $props()

	function openScheduleEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			scheduleEditor?.openNew(isFlow, path)
		} else {
			scheduleEditor?.openEdit(selectedTrigger.path, isFlow, false)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'schedule' &&
			scheduleEditor &&
			openScheduleEditor(isFlow, selectedTrigger.isDraft ?? false)
	})

	$inspect('dbg inspect', selectedTrigger?.isDraft)
</script>

<ScheduleEditor
	useDrawer={false}
	bind:this={scheduleEditor}
	on:update-config
	on:update
	hideTarget
	useEditButton
	preventSave={!isDeployed}
>
	{#snippet description()}
		<div class="flex flex-col gap-2 pb-4">
			<Description link="https://www.windmill.dev/docs/core_concepts/scheduling">
				Run scripts and flows automatically on a recurring basis using cron expressions.
			</Description>

			{#if !isDeployed}
				<Alert
					title={`Deploy the ${isFlow ? 'flow' : 'script'} to save the schedule`}
					type="info"
					size="xs"
				/>
			{/if}
		</div>
	{/snippet}
</ScheduleEditor>
<!-- hideTarget
	hidePath
    {header} -->
