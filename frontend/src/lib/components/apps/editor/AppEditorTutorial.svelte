<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import AppTutorials from '../../AppTutorials.svelte'
	import { skipAllTodos } from '$lib/tutorialUtils'
	import { BookOpen } from 'lucide-svelte'

	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import TutorialItem from '$lib/components/tutorials/TutorialItem.svelte'

	let targetTutorial: string | undefined = undefined
	let appTutorials: AppTutorials | undefined = undefined
</script>

<ButtonDropdown hasPadding={false}>
	<svelte:fragment slot="buttonReplacement">
		<Button nonCaptureEvent size="xs" color="light">
			<div class="flex flex-row items-center">
				<BookOpen size={16} />
				Tutorials
			</div>
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="items">
		<TutorialItem
			on:click={() => appTutorials?.runTutorialById('background-runnable')}
			label="Background runnables"
			index={5}
		/>
		<TutorialItem
			on:click={() => appTutorials?.runTutorialById('background-runnable')}
			label="Connection"
			index={6}
		/>
		<TutorialItem
			on:click={() => appTutorials?.runTutorialById('background-runnable')}
			label="Expression evaluation"
			index={7}
		/>
	</svelte:fragment>
</ButtonDropdown>

<AppTutorials
	bind:this={appTutorials}
	on:error={({ detail }) => {
		targetTutorial = detail.detail
	}}
	on:skipAll={() => {
		skipAllTodos()
	}}
	on:reload
/>
