<script lang="ts">
	import { tickPainted } from '$lib/utils/paint'
	import type ScheduleEditorInner from './ScheduleEditorInner.svelte'
	import type { Schedule } from '$lib/gen'

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

	/** Edit a schedule that exists only as the caller's draft; Save hands it back. */
	export async function openDraft(
		cfg: Schedule,
		onSaveDraft: (cfg: Record<string, any>) => boolean
	) {
		open = true
		await loadInner()
		await tickPainted()
		drawer?.openNew(false, cfg.script_path, cfg, undefined, cfg.script_path, { onSaveDraft })
	}

	let drawer: ScheduleEditorInner | undefined = $state()
</script>

{#if open}
	{#await loadInner() then Inner}
		<Inner.default {onUpdate} bind:this={drawer} />
	{/await}
{/if}
