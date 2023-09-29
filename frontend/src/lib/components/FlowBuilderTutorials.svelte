<script lang="ts">
	import { BookOpen } from 'lucide-svelte'
	import ButtonDropdown from './common/button/ButtonDropdown.svelte'
	import Button from './common/button/Button.svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'
	import FlowBuilderTutorialSimpleFlow from './tutorials/FlowBuilderTutorialSimpleFlow.svelte'
	import FlowBuilderTutorialsForLoop from './tutorials/FlowBuilderTutorialsForLoop.svelte'
	import FlowBranchOne from './tutorials/FlowBranchOne.svelte'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	$: tainted =
		$flowStore.value.modules.length > 0 || Object.keys($flowStore.schema?.properties).length > 0
</script>

{#if !tainted}
	<button on:pointerdown|stopPropagation>
		<ButtonDropdown hasPadding={false}>
			<svelte:fragment slot="buttonReplacement">
				<Button nonCaptureEvent size="xs" color="light" variant="border">
					<div class="flex flex-row gap-2 items-center">
						<BookOpen size={16} />
						Tutorials
					</div>
				</Button>
			</svelte:fragment>
			<svelte:fragment slot="items">
				<FlowBuilderTutorialSimpleFlow />
				<FlowBuilderTutorialsForLoop />
				<FlowBranchOne />
			</svelte:fragment>
		</ButtonDropdown>
	</button>
{/if}
