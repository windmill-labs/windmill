<script lang="ts">
	import { BookOpen } from 'lucide-svelte'
	import ButtonDropdown from './common/button/ButtonDropdown.svelte'
	import Button from './common/button/Button.svelte'

	import FlowBuilderTutorialSimpleFlow from './tutorials/FlowBuilderTutorialSimpleFlow.svelte'
	import FlowBuilderTutorialsForLoop from './tutorials/FlowBuilderTutorialsForLoop.svelte'
	import FlowBranchOne from './tutorials/FlowBranchOne.svelte'
	import FlowBranchAll from './tutorials/FlowBranchAll.svelte'
	import MenuItem from './common/menu/MenuItem.svelte'
	import { classNames } from '$lib/utils'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import { tutorialsToDo } from '$lib/stores'
	import { clickButtonBySelector } from './tutorials/utils'

	let buttonDropdown: ButtonDropdown | undefined = undefined

	$: {
		if (buttonDropdown && $tutorialsToDo.includes(0)) {
			clickButtonBySelector('#tutorials-button')
		}
	}
</script>

<button on:pointerdown|stopPropagation>
	<ButtonDropdown hasPadding={false} bind:this={buttonDropdown}>
		<svelte:fragment slot="buttonReplacement">
			<Button nonCaptureEvent size="xs" color="light" variant="border" id="tutorials-button">
				<div class="flex flex-row gap-2 items-center">
					<BookOpen size={16} />
					Tutorials
				</div>
			</Button>
		</svelte:fragment>
		<svelte:fragment slot="items">
			<FlowBuilderTutorialSimpleFlow on:reload on:skipAll={skipAllTodos} />
			<FlowBuilderTutorialsForLoop on:reload on:skipAll={skipAllTodos} />
			<FlowBranchOne on:reload on:skipAll={skipAllTodos} />
			<FlowBranchAll on:reload on:skipAll={skipAllTodos} />
			<div class="border-t border-surface-hover" />
			<MenuItem
				on:click={() => {
					resetAllTodos()
				}}
			>
				<div
					class={classNames(
						'text-primary flex flex-row items-center text-left gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
					)}
				>
					Reset tutorials
				</div>
			</MenuItem>
			<MenuItem on:click={() => skipAllTodos()}>
				<div
					class={classNames(
						'text-primary flex flex-row items-center text-left gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
					)}
				>
					Skip tutorials
				</div>
			</MenuItem>
		</svelte:fragment>
	</ButtonDropdown>
</button>

<style global>
	.driver-popover-title {
		@apply leading-6 text-primary text-base;
	}

	.driver-popover-description {
		@apply text-secondary text-sm;
	}

	.driver-popover {
		@apply p-6 bg-surface;
	}
</style>
