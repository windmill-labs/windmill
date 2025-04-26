<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import AppTutorials from '../../AppTutorials.svelte'
	import { BookOpen, CheckCircle, Circle, RefreshCw, CheckCheck } from 'lucide-svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import { getContext, onMount } from 'svelte'
	import type { AppViewerContext } from '../types'
	import { ignoredTutorials } from '$lib/components/tutorials/ignoredTutorials'
	import { tutorialsToDo } from '$lib/stores'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { isAppTainted } from '$lib/components/tutorials/utils'

	let appTutorials: AppTutorials | undefined = undefined
	let targetTutorial: string | undefined = undefined

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	onMount(() => {
		const urlParams = new URLSearchParams(window.location.search)

		const forkedFromTheHub = urlParams.get('hub')
		const forkedFromTemplate = urlParams.get('template')

		if (
			!isAppTainted($app) &&
			!$ignoredTutorials.includes(7) &&
			$tutorialsToDo.includes(7) &&
			!forkedFromTheHub &&
			!forkedFromTemplate
		) {
			appTutorials?.runTutorialById('simpleapptutorial')
		}
	})

	export function toggleTutorial() {
		const urlParams = new URLSearchParams(window.location.search)
		const tutorial = urlParams.get('tutorial')

		if (tutorial === 'simpleapptutorial') {
			appTutorials?.runTutorialById('simpleapptutorial')
		}
	}

	async function getTutorialItems() {
		return [
			{
				displayName: 'App tutorial',
				action: () => appTutorials?.runTutorialById('simpleapptutorial'),
				index: 7,
				icon: $tutorialsToDo.includes(7) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(7) ? undefined : 'green'
			},
			{
				displayName: 'Background runnables',
				action: () => appTutorials?.runTutorialById('backgroundrunnables'),
				index: 5,
				icon: $tutorialsToDo.includes(5) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(5) ? undefined : 'green'
			},
			{
				displayName: 'Connection',
				action: () => appTutorials?.runTutorialById('connection'),
				index: 6,
				icon: $tutorialsToDo.includes(6) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(6) ? undefined : 'green'
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
		<svelte:fragment slot="buttonReplacement">
			<Button
				nonCaptureEvent
				size="xs"
				color="light"
				variant="border"
				iconOnly
				startIcon={{
					icon: BookOpen
				}}
			/>
		</svelte:fragment>
	</Dropdown>
{/key}

<AppTutorials
	bind:this={appTutorials}
	on:reload
	on:error={({ detail }) => {
		targetTutorial = detail.detail
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
