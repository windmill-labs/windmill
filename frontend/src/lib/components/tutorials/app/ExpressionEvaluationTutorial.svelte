<script lang="ts">
	import Tutorial from '../Tutorial.svelte'

	export let name: string
	export let index: number

	let tutorial: Tutorial | undefined = undefined

	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	{name}
	on:error
	on:skipAll
	getSteps={(driver) => [
		{
			popover: {
				title: 'Expression evaluation tutorial',
				description:
					'Learn how to build our first branch to be executed on a condition. You can use arrow keys to navigate'
			}
		}
	]}
/>
