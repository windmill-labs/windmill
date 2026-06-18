<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import FlowTutorials from '../FlowTutorials.svelte'
	import { BookOpen, CheckCircle, Circle, RefreshCw, CheckCheck } from 'lucide-svelte'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import { resetAllTodos, skipAllTodos } from '$lib/tutorialUtils'
	import { tutorialsToDo } from '$lib/stores'
	import { getTutorialIndex } from '$lib/tutorials/config'

	let flowTutorials: FlowTutorials | undefined = $state(undefined)

	async function getTutorialItems() {
		return [
			{
				displayName: 'Build a flow',
				action: () => flowTutorials?.runTutorialById('flow-live-tutorial'),
				index: getTutorialIndex('flow-live-tutorial'),
				icon: $tutorialsToDo.includes(getTutorialIndex('flow-live-tutorial')) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(getTutorialIndex('flow-live-tutorial')) ? undefined : 'green'
			},
			{
				displayName: 'Fix a broken flow',
				action: () => flowTutorials?.runTutorialById('troubleshoot-flow'),
				index: getTutorialIndex('troubleshoot-flow'),
				icon: $tutorialsToDo.includes(getTutorialIndex('troubleshoot-flow')) ? Circle : CheckCircle,
				iconColor: $tutorialsToDo.includes(getTutorialIndex('troubleshoot-flow')) ? undefined : 'green'
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

<FlowTutorials
	bind:this={flowTutorials}
	on:reload
	on:error
	on:skipAll
/>

