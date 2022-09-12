<script lang="ts">
	import { isCopyFirstStepSchemaDisabled } from '$lib/components/flows/flowState'
	import { copyFirstStepSchema } from '$lib/components/flows/flowStore'

	import { Button } from 'flowbite-svelte'
</script>

<Button
	color="light"
	size="xs"
	disabled={$isCopyFirstStepSchemaDisabled}
	on:click={copyFirstStepSchema}
>
	Copy from first step schema
</Button>
