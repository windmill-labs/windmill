<script lang="ts">
	import { tickPainted } from '$lib/utils/paint'
	import ScheduleEditorInner from './ScheduleEditorInner.svelte'
	import type { ScheduleRunsSample } from '$lib/components/schedules/scheduleDrift'

	let {
		onUpdate,
		getRunsSample
	}: {
		onUpdate?: (path?: string) => void
		getRunsSample?: (path: string) => ScheduleRunsSample | undefined
	} = $props()

	let open = $state(false)
	export async function openEdit(ePath: string, isFlow: boolean, fixedScriptPath?: string) {
		open = true
		await tickPainted()
		drawer?.openEdit(ePath, isFlow, undefined, fixedScriptPath)
	}

	export async function openNew(
		is_flow: boolean,
		initial_script_path?: string,
		schedule_path?: string,
		fixedScriptPath?: string
	) {
		open = true
		await tickPainted()
		drawer?.openNew(is_flow, initial_script_path, undefined, schedule_path, fixedScriptPath)
	}

	let drawer: ScheduleEditorInner | undefined = $state()
</script>

{#if open}
	<ScheduleEditorInner {onUpdate} {getRunsSample} bind:this={drawer} />
{/if}
