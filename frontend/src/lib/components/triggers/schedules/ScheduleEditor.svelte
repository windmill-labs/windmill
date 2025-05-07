<script lang="ts">
	import { tick } from 'svelte'
	import ScheduleEditorInner from './ScheduleEditorInner.svelte'

	let open = false
	export async function openEdit(ePath: string, isFlow: boolean) {
		open = true
		await tick()
		drawer?.openEdit(ePath, isFlow)
	}

	export async function openNew(
		is_flow: boolean,
		initial_script_path?: string,
		schedule_path?: string
	) {
		open = true
		await tick()
		drawer?.openNew(is_flow, initial_script_path, undefined, undefined, schedule_path)
	}

	let drawer: ScheduleEditorInner
</script>

{#if open}
	<ScheduleEditorInner on:update bind:this={drawer} />
{/if}
