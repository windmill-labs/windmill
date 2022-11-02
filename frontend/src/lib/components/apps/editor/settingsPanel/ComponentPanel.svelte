<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppComponent, AppEditorContext } from '../../types'

	export let component: AppComponent | undefined

	const dispatch = createEventDispatcher()
	const { staticOutputs, worldStore, schemas, app, selection } =
		getContext<AppEditorContext>('AppEditorContext')

	function deleteComponent() {
		dispatch('remove')
	}

	$: schema = component && $schemas[component.id]
</script>

<div class="p-2 flex flex-col gap-2">
	{#if component}
		{#if schema}
			<div class="text-sm font-bold">Inputs</div>

			<SchemaForm {schema} args={component.inputs.runInputs} />
		{/if}

		<Button
			size="sm"
			color="red"
			startIcon={{ icon: faTrashAlt }}
			on:click={() => deleteComponent()}
		>
			Delete component
		</Button>
	{:else}
		Empty component
	{/if}
</div>
