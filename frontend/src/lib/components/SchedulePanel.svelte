<script lang="ts">
	import ScheduleEditorInner from '$lib/components/triggers/schedules/ScheduleEditorInner.svelte'
	import Description from '$lib/components/Description.svelte'
	import { onMount } from 'svelte'
	let scheduleEditor = $state<ScheduleEditorInner | null>(null)
	let {
		selectedTrigger,
		isFlow,
		path,
		defaultValues = undefined,
		schema,
		customLabel = undefined,
		...restProps
	} = $props()

	async function openScheduleEditor(isFlow: boolean) {
		if (selectedTrigger.isNew) {
			await scheduleEditor?.openNew(isFlow, path, { ...defaultValues, path: selectedTrigger.path })
			// The autosave inside `openNew` created the draft row, so this trigger is
			// no longer new: re-selecting it must reload that row, not reset the form
			// to defaults and overwrite what the user just configured.
			selectedTrigger.isNew = false
			selectedTrigger.newTriggerSeed = undefined
		} else {
			scheduleEditor?.openEdit(selectedTrigger.path, isFlow, defaultValues)
		}
	}

	onMount(() => {
		selectedTrigger?.type === 'schedule' &&
			scheduleEditor &&
			openScheduleEditor(isFlow)
	})
</script>

<ScheduleEditorInner
	useDrawer={false}
	bind:this={scheduleEditor}
	hideTarget
	allowDraft
	trigger={selectedTrigger}
	draftSchema={schema}
	{customLabel}
	{...restProps}
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
