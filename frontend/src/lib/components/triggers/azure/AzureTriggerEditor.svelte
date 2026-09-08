<script lang="ts">
	import { tickPainted } from '$lib/utils/paint'
	import AzureTriggerEditorInner from './AzureTriggerEditorInner.svelte'

	let { onUpdate }: { onUpdate?: (path?: string) => void } = $props()

	let open = $state(false)
	export async function openEdit(ePath: string, isFlow: boolean) {
		open = true
		await tickPainted()
		drawer?.openEdit(ePath, isFlow)
	}

	export async function openNew(
		is_flow: boolean,
		initial_script_path?: string,
		defaultValues?: Record<string, any>
	) {
		open = true
		await tickPainted()
		drawer?.openNew(is_flow, initial_script_path, defaultValues)
	}

	let drawer: AzureTriggerEditorInner | undefined = $state()
</script>

{#if open}
	<AzureTriggerEditorInner {onUpdate} bind:this={drawer} />
{/if}
