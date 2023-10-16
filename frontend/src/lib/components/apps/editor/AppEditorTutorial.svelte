<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import AppTutorials from '../../AppTutorials.svelte'
	import { BookOpen } from 'lucide-svelte'

	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import TutorialItem from '$lib/components/tutorials/TutorialItem.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'
	import { classNames } from '$lib/utils'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'

	let appTutorials: AppTutorials | undefined = undefined
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

<AppTutorials bind:this={appTutorials} on:reload />

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
