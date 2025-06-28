<script lang="ts">
	import { FlaskConical } from 'lucide-svelte'
	import { Tooltip } from '$lib/components/meltComponents'
</script>

<Tooltip>
	<FlaskConical size={14} class="text-tertiary inline-block" />
	<span slot="text">Config used for creating a testing endpoint</span>
</Tooltip>
