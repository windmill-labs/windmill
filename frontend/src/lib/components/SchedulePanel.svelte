<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'
	import Description from '$lib/components/Description.svelte'

	let scheduleEditor = $state<ScheduleEditor | null>(null)
	let { selectedTrigger, isFlow, path } = $props()

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
>
	{#snippet description()}
		<Description link="https://www.windmill.dev/docs/core_concepts/scheduling" class="mb-4">
			Run scripts and flows automatically on a recurring basis using cron expressions. Each script
			or flow can have multiple schedules, with one designated as primary.
		</Description>
	{/snippet}
</ScheduleEditor>
<!-- hideTarget
	hidePath
    {header} -->
