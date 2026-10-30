<script lang="ts">
	import { Button } from '$lib/components/common'
	import { UserPlus } from 'lucide-svelte'

	interface Props {
		/** Sharing needs a deployed item: its access lives on the deployed row. */
		deployed: boolean
		iconOnly?: boolean
		unifiedSize?: 'sm' | 'md'
		onclick: () => void
	}

	let { deployed, iconOnly = false, unifiedSize = 'md', onclick }: Props = $props()
</script>

<Button
	variant="default"
	{unifiedSize}
	startIcon={{ icon: UserPlus }}
	{iconOnly}
	disabled={!deployed}
	title={deployed ? 'Share' : 'Deploy once to share it'}
	{onclick}
>
	Share
</Button>
