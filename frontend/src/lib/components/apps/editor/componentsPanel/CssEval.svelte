<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import InputsSpecEditor from '../settingsPanel/InputsSpecEditor.svelte'

	interface Props {
		evalClass: RichConfiguration
		key: string
	}

	let { evalClass = $bindable(), key }: Props = $props()

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	let id = $derived(Array.isArray($selectedComponent) ? $selectedComponent[0] : $selectedComponent)
</script>

{#if evalClass && id}
	<InputsSpecEditor
		bind:componentInput={evalClass}
		{id}
		userInputEnabled={false}
		shouldCapitalize={true}
		resourceOnly={false}
		fieldType="text"
		customTitle="Dynamic class (eval)"
		displayType={false}
		placeholder={undefined}
		format={undefined}
		selectOptions={undefined}
		subFieldType={undefined}
		allowTypeChange={false}
		{key}
	/>
{/if}
