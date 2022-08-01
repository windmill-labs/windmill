<script lang="ts">
	import { Button } from 'flowbite-svelte'
	import { copyFirstStepSchema, isCopyFirstStepSchemaDisabled } from './flowStore'
</script>

<Button
	color="alternative"
	size="sm"
	disabled={$isCopyFirstStepSchemaDisabled}
	on:click={copyFirstStepSchema}
>
	Copy from first step schema
</Button>
