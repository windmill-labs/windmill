<script lang="ts">
	import Tutorial from '../Tutorial.svelte'

	interface Props {
		name: string;
		index: number;
		onerror?: (...args: any[]) => any
		onskipAll?: (...args: any[]) => any
	}

	let { name, index,
		onerror = undefined,
		onskipAll = undefined }: Props = $props();

	let tutorial: Tutorial | undefined = $state(undefined)

	export function runTutorial() {
		tutorial?.runTutorial()
	}
</script>

<Tutorial
	bind:this={tutorial}
	{index}
	{name}
	onerror={onerror}
	onskipAll={onskipAll}
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
