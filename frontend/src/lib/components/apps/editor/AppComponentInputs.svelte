<script lang="ts">
	import type { ConnectedAppInput, RowAppInput, StaticAppInput, UserAppInput } from '../inputType'
	import type { AppComponent } from '../types'
	import InputsSpecsEditor from './settingsPanel/InputsSpecsEditor.svelte'

	export let fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	export let component: AppComponent
	export let resourceOnly: boolean = false

	$: fields = resourceOnly ? fields : fields
</script>

<InputsSpecsEditor
	id={component.id}
	shouldCapitalize={false}
	bind:inputSpecs={fields}
	userInputEnabled={component.type !== 'buttoncomponent'}
/>
