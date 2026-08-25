<script lang="ts">
	import { tickPainted } from '$lib/utils/paint'
	import ScheduleEditorInner from './ScheduleEditorInner.svelte'

	let { onUpdate }: { onUpdate?: (path?: string) => void } = $props()

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
	<ScheduleEditorInner {onUpdate} bind:this={drawer} />
{/if}
