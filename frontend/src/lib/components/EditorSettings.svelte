<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import FormatOnSave from './FormatOnSave.svelte'
	import VimMode from './VimMode.svelte'
	import RelativeLineNumbers from './RelativeLineNumbers.svelte'
	import { Button } from './common'
	import CodeCompletionStatus from './copilot/CodeCompletionStatus.svelte'
	import type { EditorBarUi } from './custom_ui'
	import Popover from './meltComponents/Popover.svelte'
	import type { ComponentProps } from 'svelte'

	interface Props {
		customUi?: EditorBarUi
		btnProps?: ComponentProps<typeof Button>
	}

	let { customUi = {}, btnProps }: Props = $props()
</script>

{#if customUi?.autoformatting != false || customUi?.vimMode != false || customUi?.aiCompletion != false}
	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		usePointerDownOutside
		contentClasses="flex flex-col gap-y-2 p-4"
	>
		{#snippet trigger()}
			{#if customUi.editorSettings != false}
				<Button
					nonCaptureEvent={true}
					startIcon={{ icon: Settings }}
					iconOnly
					title="Editor settings"
					{...btnProps}
				/>
			{/if}
		{/snippet}

		{#snippet content()}
			{#if customUi?.autoformatting != false}
				<div>
					<FormatOnSave />
				</div>
			{/if}
			{#if customUi?.vimMode != false}
				<div>
					<VimMode />
				</div>
			{/if}
			{#if customUi?.relativeLineNumbers != false}
				<div>
					<RelativeLineNumbers />
				</div>
			{/if}
			{#if customUi?.aiCompletion != false}
				<div>
					<CodeCompletionStatus />
				</div>
			{/if}
		{/snippet}
	</Popover>
{/if}
