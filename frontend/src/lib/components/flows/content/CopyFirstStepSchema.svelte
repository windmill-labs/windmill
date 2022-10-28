<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { copyFirstStepSchema, flowStore } from '../flowStore'
	import { isEmptyFlowModule } from '../utils'
</script>

<Button
	color="light"
	size="xs"
	disabled={$flowStore.value.modules.length === 0 || isEmptyFlowModule($flowStore.value.modules[0])}
	on:click={copyFirstStepSchema}
	variant="border"
>
	Copy from first step schema
</Button>
