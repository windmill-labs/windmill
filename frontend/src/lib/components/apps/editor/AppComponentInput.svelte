<script lang="ts">
	import { components, type AppComponent } from './component'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	interface Props {
		component: AppComponent
		resourceOnly?: boolean
	}

	let { component = $bindable(), resourceOnly = false }: Props = $props()
</script>

{#if component?.componentInput?.type === 'runnable' && Object.keys(component?.componentInput?.fields ?? {}).length > 0}
	<div class="mb-8 border p-2">
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
			<span class="text-sm text-secondary">No resource input</span>
		{:else}
			<InputsSpecsEditor
				id={component.id}
				shouldCapitalize={false}
				bind:inputSpecs={component.componentInput.fields}
				userInputEnabled={component.type === 'formcomponent' ||
					component.type === 'formbuttoncomponent'}
				{resourceOnly}
			/>
		{/if}
	</div>
{/if}
