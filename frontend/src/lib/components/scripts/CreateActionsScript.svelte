<script lang="ts">
	import { Code2, Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { base } from '$lib/base'

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
		href="{base}/scripts/add"
		endIcon={{ icon: Code2 }}
	>
		Script
	</Button>
</div>
