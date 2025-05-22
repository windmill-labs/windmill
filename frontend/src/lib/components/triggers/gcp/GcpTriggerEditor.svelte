<script lang="ts">
	import { tick } from 'svelte'
	import GcpTriggerEditorInner from './GcpTriggerEditorInner.svelte'

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
		defaultValues?: Record<string, any>,
		newDraft?: boolean
	) {
		open = true
		await tick()
		drawer?.openNew(is_flow, initial_script_path, defaultValues)
	}

	let drawer: GcpTriggerEditorInner | undefined = $state()
</script>

{#if open}
	<GcpTriggerEditorInner {onUpdate} bind:this={drawer} />
{/if}
