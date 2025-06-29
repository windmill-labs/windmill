<script lang="ts">
	import { untrack } from 'svelte'

	type InputType = string | number | boolean
	interface Props {
		input?: HTMLInputElement | undefined
		defaultValue?: InputType | undefined
	}

	let { input = $bindable(undefined), defaultValue = undefined }: Props = $props()

	function setInputValueToDefaultValue() {
		if (defaultValue !== undefined && input) {
			input.value = String(defaultValue)
		}
	}

	function clearInputValue() {
		if (input) {
			input.value = ''
		}
	}

	$effect.pre(() => {
		input && defaultValue && untrack(() => setInputValueToDefaultValue())
	})
	$effect.pre(() => {
		input &&
			(defaultValue === '' || defaultValue === undefined || defaultValue === null) &&
			untrack(() => clearInputValue())
	})
</script>
