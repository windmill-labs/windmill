<script lang="ts">
	import ScheduleEditorInner from '$lib/components/triggers/schedules/ScheduleEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { onMount } from 'svelte'
	let scheduleEditor = $state<ScheduleEditorInner | null>(null)
	let {
		selectedTrigger,
		isFlow,
		path,
		isDeployed = false,
		defaultValues = undefined,
		newDraft = false,
		edit = false,
		schema,
		isEditor = false,
		customLabel = undefined
	} = $props()

	function openScheduleEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			scheduleEditor?.openNew(isFlow, path, defaultValues, newDraft)
		} else {
			scheduleEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		selectedTrigger?.type === 'schedule' &&
			scheduleEditor &&
			openScheduleEditor(isFlow, selectedTrigger.isDraft ?? false)
	})
</script>

<ScheduleEditorInner
	useDrawer={false}
	bind:this={scheduleEditor}
	on:update-config
	on:update
	on:save-draft
	on:reset
	on:toggle-edit-mode
	on:delete
	hideTarget
	allowDraft
	hasDraft={!!selectedTrigger.draftConfig}
	isDraftOnly={selectedTrigger.isDraft}
	editMode={edit}
	primary={selectedTrigger.isPrimary}
	draftSchema={schema}
	{isEditor}
	{customLabel}
	{isDeployed}
>
	{#snippet docDescription()}
		<div class="flex flex-col gap-2 pb-4">
			<Description link="https://www.windmill.dev/docs/core_concepts/scheduling">
				Run scripts and flows automatically on a recurring basis using cron expressions.
			</Description>
		</div>
	{/snippet}
</ScheduleEditorInner>
<!-- hideTarget
	hidePath
    {header} -->
