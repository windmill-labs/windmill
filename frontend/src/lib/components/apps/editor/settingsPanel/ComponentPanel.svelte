<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import { faTrashAlt } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher, getContext } from 'svelte'

	import type { AppComponent, AppEditorContext } from '../../types'
	import InputsSpecsEditor from './InputsSpecsEditor.svelte'
	import { isPolicyDefined } from './utils'
	export let component: AppComponent | undefined

	const dispatch = createEventDispatcher()
	const { app } = getContext<AppEditorContext>('AppEditorContext')
</script>

<span class="text-sm font-bold border-y w-full py-1 px-2">Component editor</span>

<div class="p-2 flex flex-col gap-2 items-start">
	{#if component}
		<div class="text-sm font-bold">Inputs</div>
		{#if component.type === 'runformcomponent' && component.inputs}
			<InputsSpecsEditor inputSpecs={component.inputs} componenId={component.id} />

			{#if isPolicyDefined($app, component.id)}
				isPolicyDefined
			{:else}
				Should display hardcoded form to set type and path (script picker)
			{/if}
		{/if}
		{#if component.type === 'displaycomponent' && component.inputs}
			<InputsSpecsEditor inputSpecs={component.inputs} componenId={component.id} />
		{/if}

		<Button
			size="xs"
			color="red"
			startIcon={{ icon: faTrashAlt }}
			on:click={() => dispatch('remove')}
		>
			Delete component
		</Button>
	{:else}
		Empty component
	{/if}
</div>
