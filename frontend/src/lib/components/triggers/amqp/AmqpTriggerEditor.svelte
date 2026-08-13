<script lang="ts">
	import { tick } from 'svelte'
	import AmqpTriggerEditorInner from './AmqpTriggerEditorInner.svelte'

	let { onUpdate }: { onUpdate?: (path?: string) => void } = $props()

	let open = $state(false)
	export async function openEdit(ePath: string, isFlow: boolean, fixedScriptPath?: string) {
		open = true
		await tick()
		drawer?.openEdit(ePath, isFlow, undefined, fixedScriptPath)
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

	let drawer: AmqpTriggerEditorInner | undefined = $state()
</script>

{#if open}
	<AmqpTriggerEditorInner {onUpdate} bind:this={drawer} />
{/if}
