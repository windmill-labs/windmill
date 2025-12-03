<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Save } from 'lucide-svelte'

	let { saveToWorkspace }: { saveToWorkspace: (path: string) => void } = $props()
</script>

<Button
	size="xs"
	color="light"
	startIcon={{ icon: Save }}
	on:click={(e) => saveToWorkspace('')}
	iconOnly={false}
>
	Save to workspace
</Button>
