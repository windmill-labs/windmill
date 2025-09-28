<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Plug } from 'lucide-svelte'

	interface Props {
		label?: string
		onClick: () => void
		connecting?: boolean
		class?: string
	}

	let {
		label = 'Add object from an expression',
		onClick,
		connecting = false,
		class: className = ''
	}: Props = $props()
</script>

{#if !connecting}
	<Button
		variant="border"
		color="light"
		size="xs"
		btnClasses="font-normal text-tertiary"
		startIcon={{ icon: Plug }}
		onclick={onClick}
		wrapperClasses={className}
	>
		{label}
	</Button>
{/if}
