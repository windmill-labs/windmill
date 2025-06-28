<script lang="ts">
	import { tick } from 'svelte'
	import RouteEditorInner from './RouteEditorInner.svelte'
	import type { EditHttpTrigger, HttpTrigger } from '$lib/gen'

	interface Props {
		onUpdate?: (cfg?: Record<string, any>) => void
		customSaveBehavior?: (cfg: HttpTrigger | EditHttpTrigger) => void
	}

	let { onUpdate = undefined, customSaveBehavior }: Props = $props()

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
	<RouteEditorInner {customSaveBehavior} {onUpdate} bind:this={drawer} />
{/if}
