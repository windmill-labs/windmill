<script lang="ts">
	import type { ButtonType } from '$lib/components/common/button/model'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/Popover.svelte'

	type ActionType = {
		label: string
		icon: any
		color: ButtonType.Color
		callback: () => void
	}

	interface Props {
		actions?: ActionType[]
	}

	let { actions = [] }: Props = $props()
</script>

<div class="flex flex-row gap-1 justify-end">
	{#each actions as action, index (index)}
		<Popover notClickable disappearTimeout={0}>
			{#snippet text()}
				{action.label}
			{/snippet}
			<Button color={action.color} on:click={action.callback} size="xs2" variant="border">
				<div class="flex flex-row gap-1 items-center">
					{#if action.icon}
						<action.icon size={12} />
					{/if}
				</div>
			</Button>
		</Popover>
	{/each}
</div>
