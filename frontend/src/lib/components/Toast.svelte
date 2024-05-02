<script lang="ts">
	import Button from './common/button/Button.svelte'
	import type { ToastAction } from '$lib/toast'

	export let message: string
	export let actions: ToastAction[] = []
	export let errorMessage: string | undefined = undefined
	export let onClose: () => void
</script>

<div class="w-full h-full">
	<div class="flex flex-col justify-center items-center h-full gap-2">
		<p class="text-sm text-secondary break-words">{message}</p>
		{#if errorMessage}
			<p class="text-sm text-secondary border bg-surface-secondary p-2 w-full overflow-auto mt-2">
				{errorMessage}
			</p>
		{/if}

		<div class="flex flex-row gap-2">
			{#each actions as action, index (index)}
				<Button
					on:click={() => {
						action.callback()
						onClose()
					}}
					class="text-sm !text-primary"
				>
					{action.label}
				</Button>
			{/each}
		</div>
	</div>
</div>
