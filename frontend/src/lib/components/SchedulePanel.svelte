<script lang="ts">
	import ScheduleEditor from './ScheduleEditor.svelte'

	let scheduleEditor = $state<ScheduleEditor | null>(null)
	let { selectedTrigger, isFlow, path } = $props()

	function openScheduleEditor(isFlow: boolean, isDraft: boolean) {
		if (isDraft) {
			scheduleEditor?.openNew(isFlow, path)
		} else {
			scheduleEditor?.openEdit(selectedTrigger.path, isFlow)
		}
	}

	$effect(() => {
		selectedTrigger?.type === 'schedule' &&
			scheduleEditor &&
			openScheduleEditor(isFlow, selectedTrigger.isDraft ?? false)
	})

	$inspect('dbg inspect', selectedTrigger?.path)
</script>

<ScheduleEditor useDrawer={false} bind:this={scheduleEditor} on:update-config on:update />
<!-- hideTarget
	hidePath
    {header} -->
