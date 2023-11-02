<script lang="ts">
	import { Code2, Plus } from 'lucide-svelte'
	import Button from '../common/button/Button.svelte'
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<Button size="sm" spacingSize="xl" startIcon={{ icon: Plus }} href="/scripts/add">
		Script <Code2 class="ml-1.5" size={18} />
	</Button>
</div>
