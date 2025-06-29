<script lang="ts">
	import type { InputType, StaticInput } from '../../inputType'
	import StaticInputEditor from './inputEditor/StaticInputEditor.svelte'

	interface Props {
		value: any
		componentInput: StaticInput<any>
		subFieldType: InputType | undefined
		id: string | undefined
	}

	let { value = $bindable(), componentInput = $bindable(), subFieldType, id }: Props = $props()

	// Bubble up changes to the real componentInput
</script>

<StaticInputEditor
	{id}
	fieldType={subFieldType}
	bind:componentInput={
		() => {
			return {
				...componentInput,
				value
			}
		},
		(v) => {
			value = v.value
		}
	}
	on:remove
/>
