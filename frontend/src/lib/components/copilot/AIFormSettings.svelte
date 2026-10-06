<script lang="ts">
	import Label from '../Label.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { copilotInfo } from '$lib/aiStore'

	interface Props {
		prompt?: string | undefined
		type?: 'flow' | 'script'
	}

	let { prompt = $bindable(), type = 'script' }: Props = $props()
</script>

{#if !$copilotInfo.workspaceDisabled}
	<Label
		label="Additional prompt for AI"
		tooltip="An AI choosing this {type}'s inputs already reads its description and each field's description. Anything written here is given to it as extra guidance: mention specific fields and how they interact."
	>
		<TextInput
			underlyingInputEl="textarea"
			bind:value={() => prompt ?? '', (v) => (prompt = v === '' ? undefined : String(v))}
			inputProps={{
				placeholder: `Instructions for the AI about how to choose this ${type}'s inputs`,
				rows: 3
			}}
		/>
	</Label>
{/if}
