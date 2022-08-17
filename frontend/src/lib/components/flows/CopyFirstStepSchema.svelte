<script lang="ts">
	import { Button } from 'flowbite-svelte'
	import { isCopyFirstStepSchemaDisabled } from './flowState'
	import { copyFirstStepSchema } from './flowStore'
</script>

<Button
	color="light"
	size="xs"
	disabled={$isCopyFirstStepSchemaDisabled}
	on:click={copyFirstStepSchema}
>
	Copy from first step schema
</Button>
