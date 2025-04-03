<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { Button } from './common'
	import AnimatedButton from './common/button/AnimatedButton.svelte'

	export let connecting: boolean
	export let id: undefined | string = undefined
</script>

<AnimatedButton animate={connecting} baseRadius="6px" animationDuration="2s" marginWidth="2px">
	<Button
		variant="border"
		color="light"
		size="xs2"
		btnClasses={connecting ? 'text-blue-500' : 'text-primary'}
		on:click
		{...id ? { id } : {}}
	>
		<Plug size={16} /> &rightarrow;
	</Button>
</AnimatedButton>
