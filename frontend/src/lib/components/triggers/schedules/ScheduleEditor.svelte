<script lang="ts">
	import { tickPainted } from '$lib/utils/paint'
	import type ScheduleEditorInner from './ScheduleEditorInner.svelte'

	let { onUpdate }: { onUpdate?: (path?: string) => void } = $props()

	// Dynamic: every script and flow row embeds this wrapper, and the editor statically
	// reaches ~280 modules (script picker, flow graph, ...).
	let inner: Promise<typeof import('./ScheduleEditorInner.svelte')> | undefined
	const loadInner = () => (inner ??= import('./ScheduleEditorInner.svelte'))

	let open = $state(false)
	export async function openEdit(ePath: string, isFlow: boolean, fixedScriptPath?: string) {
		open = true
		await loadInner()
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
		await loadInner()
		await tickPainted()
		drawer?.openNew(is_flow, initial_script_path, undefined, schedule_path, fixedScriptPath)
	}

	let drawer: ScheduleEditorInner | undefined = $state()
</script>

{#if open}
	{#await loadInner() then Inner}
		<Inner.default {onUpdate} bind:this={drawer} />
	{/await}
{/if}
