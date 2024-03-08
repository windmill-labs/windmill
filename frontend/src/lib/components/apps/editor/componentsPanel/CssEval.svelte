<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import InputsSpecEditor from '../settingsPanel/InputsSpecEditor.svelte'

	export let evalClass: RichConfiguration
	export let key: string

	const { selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	$: id = Array.isArray($selectedComponent) ? $selectedComponent[0] : $selectedComponent
</script>

{#if evalClass && id}
	<InputsSpecEditor
		bind:componentInput={evalClass}
		{id}
		userInputEnabled={false}
		shouldCapitalize={true}
		resourceOnly={false}
		fieldType="text"
		tooltip="Eval an expression that return a list of class as string to dynamically add classes to the component. The styling can then be dynamic using the global CSS Editor."
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
