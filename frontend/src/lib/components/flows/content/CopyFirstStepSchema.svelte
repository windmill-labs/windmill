<script lang="ts">
	import { Button } from 'flowbite-svelte'
	import { isEmptyFlowModule } from '../flowStateUtils'
	import { copyFirstStepSchema, flowStore } from '../flowStore'
</script>

<Button
	color="light"
	size="xs"
	disabled={$flowStore.value.modules.length === 0 || isEmptyFlowModule($flowStore.value.modules[0])}
	on:click={copyFirstStepSchema}
>
	Copy from first step schema
</Button>
