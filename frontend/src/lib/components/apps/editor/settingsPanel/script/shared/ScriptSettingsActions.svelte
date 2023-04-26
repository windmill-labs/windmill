<script lang="ts">
	import type { ButtonType } from '$lib/components/common/button/model'
	import Button from '$lib/components/common/button/Button.svelte'

	type ActionType = {
		label: string
		icon: any
		color: ButtonType.Color
		callback: () => void
	}

	export let actions: ActionType[] = []
</script>

<div class="flex flex-row gap-2 justify-end">
	{#each actions as action, index (index)}
		<Button color={action.color} on:click={action.callback} size="xs">
			<div class="flex flex-row gap-1 items-center">
				{#if action.icon}
					<action.icon size={16} />
				{/if}
				{action.label}
			</div>
		</Button>
	{/each}
</div>
