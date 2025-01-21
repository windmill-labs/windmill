<script lang="ts">
	import { tick } from 'svelte'
	import DatabaseTriggerEditorInner from './DatabaseTriggerEditorInner.svelte'

	let open = false
	export async function openEdit(ePath: string, isFlow: boolean) {
		open = true
		await tick()
		drawer?.openEdit(ePath, isFlow)
	}

	export async function openNew(is_flow: boolean, initial_script_path?: string) {
		open = true
		await tick()
		drawer?.openNew(is_flow, initial_script_path)
	}

	let drawer: DatabaseTriggerEditorInner
</script>

{#if open}
	<DatabaseTriggerEditorInner on:update bind:this={drawer} />
{/if}
