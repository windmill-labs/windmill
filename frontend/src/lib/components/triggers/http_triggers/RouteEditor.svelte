<script lang="ts">
	import { tick } from 'svelte'
	import RouteEditorInner from './RouteEditorInner.svelte'

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

	export async function getTriggers() {
		return drawer?.getTriggers()
	}

	let drawer: RouteEditorInner
</script>

{#if open}
	<RouteEditorInner on:update bind:this={drawer} />
{/if}
