<script lang="ts">
	import { Settings } from 'lucide-svelte'
	import FormatOnSave from './FormatOnSave.svelte'
	import VimMode from './VimMode.svelte'
	import { Button, Popup } from './common'
	import CodeCompletionStatus from './copilot/CodeCompletionStatus.svelte'
	import type { EditorBarUi } from './custom_ui'

	export let customUi: EditorBarUi = {}
</script>

{#if customUi?.autoformatting != false || customUi?.vimMode != false || customUi?.aiCompletion != false}
	<Popup
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	>
		<svelte:fragment slot="button">
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

		<div class="flex flex-col gap-y-2">
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
		</div>
	</Popup>
{/if}
