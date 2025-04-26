<script lang="ts">
	import { BookOpen, CheckCircle, Circle, RefreshCw, CheckCheck } from 'lucide-svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import Button from './common/button/Button.svelte'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import FlowTutorials from './FlowTutorials.svelte'
	import { tutorialsToDo } from '$lib/stores'

	let targetTutorial: string | undefined = undefined
	let flowTutorials: FlowTutorials | undefined = undefined

	async function getTutorialItems() {
		const tutorials = [
			{ displayName: 'Simple flow tutorials', id: 'action' },
			{ displayName: 'For loops tutorial', id: 'forloop' },
			{ displayName: 'Branch one tutorial', id: 'branchone' },
			{ displayName: 'Branch all tutorial', id: 'branchall' },
			{ displayName: 'Error handler', id: 'error-handler' }
		]

		return [
			...tutorials.map((tutorial, index) => ({
				displayName: tutorial.displayName,
				action: () => flowTutorials?.runTutorialById(tutorial.id),
				icon: $tutorialsToDo.includes(index) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(index) ? undefined : 'green'
			})),
			{
				displayName: 'Skip tutorials',
				action: () => skipAllTodos(),
				icon: CheckCheck
			},
			{
				displayName: 'Reset tutorials',
				action: () => resetAllTodos(),
				icon: RefreshCw
			}
		]
	}
</script>

{#key $tutorialsToDo}
	<Dropdown items={getTutorialItems} class="w-fit">
		<svelte:fragment slot="buttonReplacement">
			<Button
				nonCaptureEvent
				size="xs"
				color="light"
				variant="border"
				id="tutorials-button"
				startIcon={{ icon: BookOpen }}
			/>
		</svelte:fragment>
	</Dropdown>
{/key}

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
