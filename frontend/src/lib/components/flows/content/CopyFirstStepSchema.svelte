<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faClipboard } from '@fortawesome/free-solid-svg-icons'
	import { isEmptyFlowModule } from '../flowStateUtils'
	import { copyFirstStepSchema, flowStore } from '../flowStore'
</script>

<Button
	color="light"
	size="xs"
	disabled={$flowStore.value.modules.length === 0 || isEmptyFlowModule($flowStore.value.modules[0])}
	on:click={copyFirstStepSchema}
	variant="border"
	startIcon={{ icon: faClipboard }}
>
	Copy from first step schema
</Button>
