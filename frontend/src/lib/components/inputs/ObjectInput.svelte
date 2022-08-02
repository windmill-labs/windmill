<script lang="ts">
	import { Textarea } from 'flowbite-svelte'

	export let value: any
	export let placeholder: string = ''

	let rawValue: string | undefined = undefined
	let error: string
	$: {
		if (rawValue) {
			try {
				value = JSON.parse(rawValue)
			} catch (err) {
				error = err.toString()
			}
		}
	}
</script>

<Textarea {placeholder} bind:value={rawValue} rows={4} label="TODO should replace" />
