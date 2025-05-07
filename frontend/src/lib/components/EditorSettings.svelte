<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import FormatOnSave from './FormatOnSave.svelte'
	import VimMode from './VimMode.svelte'
	import { Button } from './common'
	import CodeCompletionStatus from './copilot/CodeCompletionStatus.svelte'
	import type { EditorBarUi } from './custom_ui'
	import Popover from './meltComponents/Popover.svelte'

	export let customUi: EditorBarUi = {}
</script>

{#if customUi?.autoformatting != false || customUi?.vimMode != false || customUi?.aiCompletion != false}
	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		usePointerDownOutside
		contentClasses="flex flex-col gap-y-2 p-4"
	>
		<svelte:fragment slot="trigger">
			<Button
				btnClasses="text-tertiary"
				color="light"
				size="xs"
				nonCaptureEvent={true}
				startIcon={{ icon: Settings }}
				iconOnly
				title="Editor settings"
			/>
		</svelte:fragment>

		<svelte:fragment slot="content">
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
			{#if customUi?.aiCompletion != false}
				<div>
					<CodeCompletionStatus />
				</div>
			{/if}
		</svelte:fragment>
	</Popover>
{/if}
