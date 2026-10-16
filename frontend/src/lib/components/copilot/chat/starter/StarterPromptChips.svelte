<script lang="ts">
	import { Button } from '$lib/components/common'
	import type { StarterPrompt } from '$lib/onboardingProfile'

	let {
		prompts,
		onPick
	}: {
		prompts: StarterPrompt[]
		onPick: (prompt: string) => void
	} = $props()
</script>

<div class="flex flex-row flex-wrap items-center gap-1.5">
	{#each prompts as example (example.label)}
		<Button
			variant="default"
			unifiedSize="xs"
			btnClasses="!rounded-full !text-2xs !text-hint"
			onClick={() => onPick(example.prompt)}
		>
			{example.label}
		</Button>
	{/each}
</div>
