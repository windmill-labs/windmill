<script lang="ts">
	import { wsBase } from '$lib/workspaceUrl'
	import { Code2, Plus } from 'lucide-svelte'
	import { Button } from '$lib/components/common'

	let { aiId, aiDescription }: { aiId: string; aiDescription: string } = $props()
</script>

<div class="flex flex-row gap-2">
	<Button
		id="create-script-button"
		{aiId}
		{aiDescription}
		unifiedSize="lg"
		variant="accent"
		startIcon={{ icon: Plus }}
		href="{$wsBase}/scripts/add"
		endIcon={{ icon: Code2 }}
	>
		Script
	</Button>
</div>
