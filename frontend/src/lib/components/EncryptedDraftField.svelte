<script lang="ts">
	import Popover from './meltComponents/Popover.svelte'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import { RotateCcw } from 'lucide-svelte'

	let {
		onReset,
		disabled = false
	}: {
		/** Clear the encrypted value so a new secret can be typed. */
		onReset: () => void
		/** Read-only contexts: the Reset action is hidden. */
		disabled?: boolean
	} = $props()
</script>

<!-- Stand-in for a secret field whose draft value was encrypted server-side
($encrypted: marker) and therefore can't be displayed or edited. -->
<Popover
	openOnHover
	debounceDelay={150}
	placement="top-start"
	class="w-full cursor-default"
	contentClasses="max-w-xs p-3"
>
	{#snippet trigger()}
		<TextInput value="" inputProps={{ placeholder: 'Encrypted', disabled: true }} />
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col items-start gap-2 text-xs text-primary font-normal text-left">
			<p>
				This secret was encrypted when your draft was saved and cannot be loaded back. The last
				saved value will be used as-is when you save.
			</p>
			{#if !disabled}
				<Button
					variant="default"
					unifiedSize="xs"
					startIcon={{ icon: RotateCcw }}
					onClick={onReset}
				>
					Reset
				</Button>
			{/if}
		</div>
	{/snippet}
</Popover>
