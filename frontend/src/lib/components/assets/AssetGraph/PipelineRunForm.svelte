<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { emptySchema } from '$lib/utils'

	interface Props {
		// The script's schema to render inputs for. SchemaForm normalizes/mutates
		// its schema in place, so we clone it into local state below.
		schema: any
		args: Record<string, any>
		isValid: boolean
	}

	let { schema, args = $bindable(), isValid = $bindable() }: Props = $props()

	// Mutable clone SchemaForm can normalize without touching the parent's script.
	// Seeded ONCE per mount — the parent remounts this via `{#key <schema content>}`
	// when a local edit adds/removes args, so the clone reseeds without a prop→state
	// $effect, and an unchanged re-resolve (a WS bundle with the same schema) keeps
	// in-progress input.
	// svelte-ignore state_referenced_locally
	let formSchema = $state<any>(structuredClone($state.snapshot(schema) ?? emptySchema()))
</script>

<SchemaForm bind:schema={formSchema} bind:args bind:isValid compact />
