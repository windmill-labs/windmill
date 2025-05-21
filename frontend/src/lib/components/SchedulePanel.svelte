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

	function openScheduleEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			scheduleEditor?.openNew(isFlow, path, defaultValues)
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
	hideTarget
	allowDraft
	hasDraft={!!selectedTrigger.draftConfig}
	isDraftOnly={selectedTrigger.isDraft}
	primary={selectedTrigger.isPrimary}
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
