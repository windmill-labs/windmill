<script lang="ts">
	import { tickPainted } from '$lib/utils/paint'
	import GcpTriggerEditorInner from './GcpTriggerEditorInner.svelte'

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
		defaultValues?: Record<string, any>,
		newDraft?: boolean
	) {
		open = true
		await tickPainted()
		drawer?.openNew(is_flow, initial_script_path, defaultValues)
	}

	let drawer: GcpTriggerEditorInner | undefined = $state()
</script>

{#if open}
	<GcpTriggerEditorInner {onUpdate} bind:this={drawer} />
{/if}
