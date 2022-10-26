<script lang="ts">
	import type { Schema } from '$lib/common'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { FlowService, ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	export let runType: 'script' | 'flow'
	export let path: string
	export let args: any

	let schema: Schema | undefined = undefined

	async function load(workspace: string) {
		if (runType === 'script') {
			const script = await ScriptService.getScriptByPath({
				workspace,
				path
			})
			schema = script.schema
		} else if (runType === 'flow') {
			const flow = await FlowService.getFlowByPath({
				workspace,
				path
			})

			schema = flow.schema
		}
	}

	$: if ($workspaceStore) {
		load($workspaceStore)
	}
</script>

{#if schema}
	<SchemaForm class="h-full pt-4" {schema} bind:args />
{/if}
