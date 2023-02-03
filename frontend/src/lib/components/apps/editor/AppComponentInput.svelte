<script lang="ts">
	import { components, type AppComponent } from './Component.svelte'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	export let component: AppComponent
	export let resourceOnly = false
</script>

{#if component?.componentInput?.type === 'runnable' && Object.keys(component?.componentInput?.fields ?? {}).length > 0}
	<div class="mb-8">
		<div class="flex justify-between mb-4">
			<span class="text-sm font-bold">{component.id}</span>
			<span class="text-sm font-bold">{components[component.type].name}</span>
		</div>

		{#if resourceOnly && Object.keys(component.componentInput.fields).filter((fieldKey) => {
				if (component?.componentInput?.type === 'runnable') {
					const fields = component.componentInput?.fields
					const field = fields[fieldKey]
					return field.fieldType === 'object' && field.format?.startsWith('resource-')
				}
				return false
			}).length === 0}
			<span class="text-sm">No resource input</span>
		{:else}
			<InputsSpecsEditor
				id={component.id}
				shouldCapitalize={false}
				bind:inputSpecs={component.componentInput.fields}
				userInputEnabled={component.type !== 'buttoncomponent'}
				{resourceOnly}
			/>
		{/if}
	</div>
{/if}
