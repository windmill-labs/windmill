<script lang="ts">
	import { tick } from 'svelte'
	import RouteEditorInner from './RouteEditorInner.svelte'

	interface Props {
		onUpdate?: (cfg?: Record<string, any>) => void
	}

	let { onUpdate = undefined }: Props = $props()

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

	let drawer: RouteEditorInner | undefined = $state()
</script>

{#if open}
	<RouteEditorInner {onUpdate} bind:this={drawer} />
{/if}
