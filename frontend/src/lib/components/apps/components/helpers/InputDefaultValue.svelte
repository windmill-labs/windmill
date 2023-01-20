<script lang="ts">
	type InputType = string | number | boolean
	export let input: HTMLInputElement | undefined = undefined
	export let defaultValue: InputType | undefined = undefined

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

	$: input && defaultValue && setInputValueToDefaultValue()
	$: input &&
		(defaultValue === '' || defaultValue === undefined || defaultValue === null) &&
		clearInputValue()
</script>
