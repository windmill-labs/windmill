<script lang="ts">
	import { FlaskConical } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
</script>

<Tooltip>
	<FlaskConical size={14} class="text-primary inline-block" />
	{#snippet text()}
		<span >Config used for creating a testing endpoint</span>
	{/snippet}
</Tooltip>
