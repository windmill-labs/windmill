<script lang="ts">
	import { tick } from 'svelte'
	import ScheduleEditorInner from './ScheduleEditorInner.svelte'

	let { onUpdate }: { onUpdate?: (path?: string) => void } = $props()

	let open = $state(false)
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
		drawer?.openNew(is_flow, initial_script_path, undefined, schedule_path)
	}

	let drawer: ScheduleEditorInner | undefined = $state()
</script>

{#if open}
	<ScheduleEditorInner {onUpdate} bind:this={drawer} />
{/if}
