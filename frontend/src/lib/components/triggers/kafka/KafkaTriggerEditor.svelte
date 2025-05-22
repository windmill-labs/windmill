<script lang="ts">
	import { tick } from 'svelte'
	import KafkaTriggerEditorInner from './KafkaTriggerEditorInner.svelte'

	let open = false
	export async function openEdit(ePath: string, isFlow: boolean) {
		open = true
		await tick()
		drawer?.openEdit(ePath, isFlow)
	}

	export async function openNew(
		is_flow: boolean,
		initial_script_path?: string,
		defaultValues?: Record<string, any>
	) {
		open = true
		await tick()
		drawer?.openNew(is_flow, initial_script_path, defaultValues)
	}

	let drawer: KafkaTriggerEditorInner
</script>

{#if open}
	<KafkaTriggerEditorInner on:update bind:this={drawer} />
{/if}
