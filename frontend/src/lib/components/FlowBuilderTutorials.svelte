<script lang="ts">
	import { BookOpen } from 'lucide-svelte'
	import ButtonDropdown from './common/button/ButtonDropdown.svelte'
	import Button from './common/button/Button.svelte'

	import MenuItem from './common/menu/MenuItem.svelte'
	import { classNames } from '$lib/utils'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import TutorialItem from './tutorials/TutorialItem.svelte'
	import FlowTutorials from './FlowTutorials.svelte'

	let targetTutorial: string | undefined = undefined
	let flowTutorials: FlowTutorials | undefined = undefined
</script>

<button on:pointerdown|stopPropagation>
	<ButtonDropdown hasPadding={false}>
		<svelte:fragment slot="buttonReplacement">
			<Button
				nonCaptureEvent
				size="xs"
				color="light"
				variant="border"
				id="tutorials-button"
				startIcon={{ icon: BookOpen }}
			>
				Tutorials
			</Button>
		</svelte:fragment>
		<svelte:fragment slot="items">
			<TutorialItem
				on:click={() => flowTutorials?.runTutorialById('action')}
				label="Simple flow tutorial"
				index={0}
			/>

			<TutorialItem
				on:click={() => flowTutorials?.runTutorialById('forloop')}
				label="For loops tutorial"
				index={1}
			/>

			<TutorialItem
				on:click={() => flowTutorials?.runTutorialById('branchone')}
				label="Branch one tutorial"
				index={2}
			/>

			<TutorialItem
				on:click={() => flowTutorials?.runTutorialById('branchall')}
				label="Branch all tutorial"
				index={3}
			/>

			<TutorialItem
				on:click={() => flowTutorials?.runTutorialById('error-handler')}
				label="Error handler"
				index={4}
			/>

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

<FlowTutorials
	bind:this={flowTutorials}
	on:error={({ detail }) => {
		targetTutorial = detail.detail
	}}
	on:skipAll={() => {
		skipAllTodos()
	}}
	on:reload
/>

<ConfirmationModal
	open={targetTutorial !== undefined}
	title="Tutorial error"
	confirmationText="Open new tab"
	on:canceled={() => {
		targetTutorial = undefined
	}}
	on:confirmed={async () => {
		window.open(`/flows/add?tutorial=${targetTutorial}&nodraft=true`, '_blank')
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span> You need to create a new flow before starting the tutorial.</span>
	</div>
</ConfirmationModal>

<style global>
	.driver-popover-title {
		@apply leading-6 text-primary text-base;
	}

	.driver-popover-description {
		@apply text-secondary text-sm;
	}

	.driver-popover {
		@apply p-6 bg-surface max-w-2xl;
	}
</style>
