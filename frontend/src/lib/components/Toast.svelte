<script lang="ts">
	import Button from './common/button/Button.svelte'
	import type { ToastAction } from '$lib/toast'

	export let message: string
	export let actions: ToastAction[] = []
	export let errorMessage: string | undefined = undefined
	export let onClose: () => void
	export let duration: number | undefined = undefined

	// if duration is set, close the toast after that duration
	if (duration) {
		setTimeout(() => {
			onClose()
		}, duration)
	}
</script>

<div class="w-full flex flex-col gap-2">
	<div>
		<p class="text-xs font-semibold text-secondary break-words">{message}</p>
		{#if errorMessage}
			<p
				class=" w-full text-secondary border border-red-200 rounded-md text-xs bg-red-100 p-2 overflow-auto mt-2"
			>
				{errorMessage}
			</p>
		{/if}
	</div>
	{#if actions.length > 0}
		<div class="flex flex-col gap-2 items-start">
			{#each actions as action, index (index)}
				<Button
					on:click={() => {
						action.callback()
						onClose()
					}}
					color="light"
					variant="border"
					size="xs2"
				>
					{action.label}
				</Button>
			{/each}
		</div>
	{/if}
</div>
