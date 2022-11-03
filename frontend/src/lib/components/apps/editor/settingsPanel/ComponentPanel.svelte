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

<div class="p-2 flex flex-col gap-2 items-start">
	<span class="text-md font-bold">Component editor</span>
	{#if component}
		{#if schema}
			<div class="text-sm font-bold">Inputs</div>

			{#if component.type === 'runformcomponent' && component.inputs}
				<SchemaForm {schema} args={component.inputs.runInputs} />
			{/if}
			{#if component.type === 'displaycomponent' && component.inputs}
				<SchemaForm {schema} args={component.inputs.result} />
			{/if}
		{/if}

		<Button
			size="xs"
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
