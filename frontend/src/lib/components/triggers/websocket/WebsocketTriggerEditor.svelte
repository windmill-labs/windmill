<script lang="ts">
	import { tick } from 'svelte'
	import WebsocketTriggerEditorInner from './WebsocketTriggerEditorInner.svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		useDrawer?: boolean
		description?: Snippet | undefined
		hideTarget?: boolean
	}

	let { useDrawer = true, description = undefined, hideTarget = false }: Props = $props()

	let open = $state(false)
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

	let drawer: WebsocketTriggerEditorInner | undefined = $state(undefined)
</script>

{#if open}
	<WebsocketTriggerEditorInner
		on:update
		bind:this={drawer}
		{useDrawer}
		{description}
		{hideTarget}
	/>
{/if}
