<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import AppTutorials from '../../AppTutorials.svelte'
	import { BookOpen, CheckCircle, Circle, RefreshCw, CheckCheck } from 'lucide-svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import { tutorialsToDo } from '$lib/stores'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let appTutorials: AppTutorials | undefined = $state(undefined)
	let targetTutorial: string | undefined = $state(undefined)

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		appTutorials?.runTutorialById(id, options)
	}

	async function getTutorialItems() {
		const backgroundRunnablesIndex = getTutorialIndex('backgroundrunnables')
		const connectionIndex = getTutorialIndex('connection')
		
		return [
			{
				displayName: 'Background runnables',
				action: () => appTutorials?.runTutorialById('backgroundrunnables'),
				index: backgroundRunnablesIndex,
				icon: $tutorialsToDo.includes(backgroundRunnablesIndex) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(backgroundRunnablesIndex) ? undefined : 'green'
			},
			{
				displayName: 'Connection',
				action: () => appTutorials?.runTutorialById('connection'),
				index: connectionIndex,
				icon: $tutorialsToDo.includes(connectionIndex) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(connectionIndex) ? undefined : 'green'
			},
			{
				displayName: 'Reset tutorials',
				action: () => resetAllTodos(),
				icon: RefreshCw
			},
			{
				displayName: 'Skip tutorials',
				action: () => skipAllTodos(),
				icon: CheckCheck
			}
		]
	}
</script>

{#key $tutorialsToDo}
	<Dropdown items={getTutorialItems}>
		{#snippet buttonReplacement()}
			<Button
				nonCaptureEvent
				unifiedSize="md"
				variant="subtle"
				iconOnly
				startIcon={{ icon: BookOpen }}
			/>
		{/snippet}
	</Dropdown>
{/key}

<AppTutorials
	bind:this={appTutorials}
	on:reload
	on:error={(event: CustomEvent<{ detail: string }>) => {
		targetTutorial = event.detail.detail
	}}
/>

<ConfirmationModal
	open={targetTutorial !== undefined}
	title="Tutorial error"
	confirmationText="Open new tab"
	on:canceled={() => {
		targetTutorial = undefined
	}}
	on:confirmed={async () => {
		window.open(`/apps/add?tutorial=${targetTutorial}&nodraft=true`, '_blank')
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span> This tutorial can only be run on a new app.</span>
	</div>
</ConfirmationModal>
