<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Plug } from 'lucide-svelte'

	interface Props {
		label?: string
		onClick: () => void
		connecting?: boolean
	}

	let { label = 'Add object from an expression', onClick, connecting = false }: Props = $props()
</script>

{#if !connecting}
	<div class="mt-2 mb-2">
		<Button variant="border" color="light" size="xs" startIcon={{ icon: Plug }} onclick={onClick}>
			{label}
		</Button>
	</div>
{/if}
