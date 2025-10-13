<script lang="ts">
	import Label from '../Label.svelte'
	import Toggle from '../Toggle.svelte'
	import Tooltip from '../Tooltip.svelte'

	interface Props {
		prompt?: string | undefined
		type?: 'flow' | 'script'
	}

	let { prompt = $bindable(undefined), type = 'script' }: Props = $props()
</script>

<div class="flex flex-col gap-2">
	<Toggle
		size="sm"
		checked={prompt !== undefined}
		on:change={() => {
			if (prompt !== undefined) {
				prompt = undefined
			} else {
				prompt = ''
			}
		}}
		options={{
			right: `Enable filling ${type} inputs with AI`
		}}
	/>
	{#if prompt !== undefined}
		<Label label="Additional prompt for AI">
			{#snippet header()}
				<Tooltip>
					AI will use script description and each field description to fill the inputs form. In
					addition, any prompt passed here will be used by AI to guide it. You can mention specific
					fields and interaction between fields here.
				</Tooltip>
			{/snippet}
			<textarea bind:value={prompt} placeholder="Instructions for the AI about how to fill the form"
			></textarea>
		</Label>
	{/if}
</div>
