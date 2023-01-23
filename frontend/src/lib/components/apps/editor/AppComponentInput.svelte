<script lang="ts">
	import type { AppComponent } from '../types'
	import { displayData } from '../utils'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	export let component: AppComponent
</script>

{#if component?.componentInput?.type === 'runnable' && Object.keys(component?.componentInput?.fields ?? {}).length > 0}
	<div class="mb-16">
		<div class="flex justify-between mb-4">
			<span class="text-sm font-bold">{component.id}</span>
			<span class="text-sm font-bold">{displayData[component.type].name}</span>
		</div>
		<InputsSpecsEditor
			id={component.id}
			shouldCapitalize={false}
			bind:inputSpecs={component.componentInput.fields}
			userInputEnabled={component.type !== 'buttoncomponent'}
		/>
	</div>
{/if}
