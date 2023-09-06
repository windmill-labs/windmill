<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import InputsSpecEditor from '../settingsPanel/InputsSpecEditor.svelte'

	export let evalClass: RichConfiguration

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
		tooltip="Use any tailwind classes to style your component"
		customTitle="Dynamic class"
		displayType={false}
		placeholder={undefined}
		format={undefined}
		selectOptions={undefined}
		subFieldType={undefined}
		allowTypeChange={false}
		key=""
	/>
{/if}
