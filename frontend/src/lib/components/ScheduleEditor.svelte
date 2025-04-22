<script lang="ts">
	import { tick } from 'svelte'
	import ScheduleEditorInner from './ScheduleEditorInner.svelte'

	export let useDrawer = true
	export let hideTarget = false
	export let useEditButton = false
	export let description: undefined | (() => any) = undefined
	export let preventSave = false

	let open = false
	export async function openEdit(ePath: string, isFlow: boolean, editing: boolean = true) {
		open = true
		await tick()
		drawer?.openEdit(ePath, isFlow, editing)
	}

	export async function openNew(is_flow: boolean, initial_script_path?: string) {
		open = true
		await tick()
		drawer?.openNew(is_flow, initial_script_path)
	}

	let drawer: ScheduleEditorInner
</script>

{#if open}
	<ScheduleEditorInner
		on:update
		{useDrawer}
		bind:this={drawer}
		{hideTarget}
		{useEditButton}
		docDescription={description}
		{preventSave}
	/>
{/if}
