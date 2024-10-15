<script lang="ts">
	import FlowModuleComponent from '$lib/components/flows/content/FlowModuleComponent.svelte'
	import Label from '$lib/components/Label.svelte'
	import type { TriggerContext } from '$lib/components/triggers'
	import PrimarySchedule from '$lib/components/PrimarySchedule.svelte'
	import { getContext } from 'svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'

	export let path: string
	export let isFlow: boolean
	export let setScheduleEnabled: (path: string, enabled: boolean) => void = () => {}

	const { triggerModule, primarySchedule } = getContext<TriggerContext>('TriggerContext')

	let scheduleEditor: ScheduleEditor
</script>

<ScheduleEditor on:update={() => {}} bind:this={scheduleEditor} />

<div class="flex flex-col gap-8">
	{#if $triggerModule}
		<Label label="Schedule">
			{#if $primarySchedule}
				<PrimarySchedule
					{scheduleEditor}
					schedule={$primarySchedule}
					can_write={true}
					{path}
					{isFlow}
					{setScheduleEnabled}
				/>
			{:else}
				<div> No schedule poll triggers </div>
			{/if}
		</Label>
		<Label label="Script">
			<FlowModuleComponent
				previousModule={undefined}
				noEditor={false}
				bind:flowModule={$triggerModule}
				failureModule={false}
				preprocessorModule={false}
				scriptKind={'script'}
				scriptTemplate={'script'}
				enableAi={false}
			/>
		</Label>
	{:else}
		<div> No schedule poll triggers </div>
	{/if}
</div>
