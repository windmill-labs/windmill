<script lang="ts">
	import { copyFirstStepSchema, isCopyFirstStepSchemaDisabled } from './flowStore'
</script>

<button
	class="default-secondary-button-v2"
	disabled={$isCopyFirstStepSchemaDisabled}
	on:click={async () => await copyFirstStepSchema()}
>
	Copy from step 1's schema
</button>
