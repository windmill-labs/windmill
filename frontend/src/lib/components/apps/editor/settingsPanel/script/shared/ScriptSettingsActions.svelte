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

	export let actions: ActionType[] = []
</script>

<div class="flex flex-row gap-1 justify-end">
	{#each actions as action, index (index)}
		<Popover notClickable disappearTimeout={0}>
			<svelte:fragment slot="text">
				{action.label}
			</svelte:fragment>
			<Button color={action.color} on:click={action.callback} size="xs2" variant="border">
				<div class="flex flex-row gap-1 items-center">
					{#if action.icon}
						<svelte:component this={action.icon} size={12} />
					{/if}
				</div>
			</Button>
		</Popover>
	{/each}
</div>
