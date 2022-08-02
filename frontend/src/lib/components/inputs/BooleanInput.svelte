<script lang="ts">
	import { Checkbox } from 'flowbite-svelte'

	export let value: any
	export let disabled: boolean = false
</script>

<Checkbox bind:checked={value} {disabled}>{value === undefined ? 'Not set' : ''}</Checkbox>
