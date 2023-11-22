<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import AppTutorials from '../../AppTutorials.svelte'
	import { BookOpen } from 'lucide-svelte'

	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import TutorialItem from '$lib/components/tutorials/TutorialItem.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'
	import { classNames } from '$lib/utils'
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
</script>

<ButtonDropdown hasPadding={false}>
	<svelte:fragment slot="buttonReplacement">
		<Button nonCaptureEvent size="xs" color="light" variant="border">
			<div class="flex flex-row items-center gap-2">
				<BookOpen size={16} />
				Tutorials
			</div>
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="items">
		<TutorialItem
			on:click={() => appTutorials?.runTutorialById('simpleapptutorial')}
			label="App tutorial"
			index={7}
		/>
		<TutorialItem
			on:click={() => appTutorials?.runTutorialById('backgroundrunnables')}
			label="Background runnables"
			index={5}
		/>
		<TutorialItem
			on:click={() => appTutorials?.runTutorialById('connection')}
			label="Connection"
			index={6}
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
