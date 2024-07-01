<script lang="ts">
	import { Code2, Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
	import { base } from '$lib/navigation'
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<Button size="sm" spacingSize="xl" color="marine" startIcon={{ icon: Plus }} href="{base}/scripts/add">
		Script <Code2 class="ml-1.5" size={18} />
	</Button>
</div>
