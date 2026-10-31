<script lang="ts">
	import TextInput from '../text_input/TextInput.svelte'
	import SettingToggle from '../itemSettings/SettingToggle.svelte'
	import { copilotInfo } from '$lib/aiStore'

	interface Props {
		prompt?: string | undefined
		type?: 'flow' | 'script'
	}

	let { prompt = $bindable(), type = 'script' }: Props = $props()

	// An empty prompt is stored as undefined, so the row keeps its own open state to show the field
	// between turning the toggle on and typing.
	let open = $state(false)
</script>

{#if !$copilotInfo.workspaceDisabled}
	<SettingToggle
		setting="ai-prompt"
		label="AI prompt"
		tooltip="An AI choosing this {type}'s inputs already reads its description and each field's description. Anything written here is given to it as extra guidance: mention specific fields and how they interact."
		checked={open || !!prompt}
		onchange={(on) => {
			open = on
			if (!on) prompt = undefined
		}}
	>
		<TextInput
			underlyingInputEl="textarea"
			bind:value={() => prompt ?? '', (v) => (prompt = v === '' ? undefined : String(v))}
			inputProps={{
				placeholder: `Instructions for the AI about how to choose this ${type}'s inputs`,
				rows: 3
			}}
		/>
	</SettingToggle>
{/if}
