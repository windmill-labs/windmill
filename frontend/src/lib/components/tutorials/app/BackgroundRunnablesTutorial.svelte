<script lang="ts">
	import Tutorial from '../Tutorial.svelte'
	import { clickButtonBySelector } from '../utils'

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
				title: 'Welcome to the Windmil Flow editor',
				description:
					'Learn how to build our first branch to be executed on a condition. You can use arrow keys to navigate'
			}
		},
		{
			element: '#app-editor-runnable-panel',
			popover: {
				title: 'Runnable panel',
				description:
					'This is the runnable panel. Here you can add runnables to your app. Runnables are scripts that can be executed in the background. You can add as many runnables as you want.'
			}
		},
		{
			element: '#create-background-runnable',
			popover: {
				title: 'Create a runnable',
				description:
					'Click here to create a runnable. Runnables are scripts that can be executed in the background. You can add as many runnables as you want.',
				onNextClick: () => {
					clickButtonBySelector('#create-background-runnable')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#app-editor-empty-runnable',
			popover: {
				title: 'Empty runnable panel',
				description:
					'This is the empty runnable panel. Here you can add runnables to your app. Runnables are scripts that can be executed in the background. You can add as many runnables as you want.'
			}
		},
		{
			element: '#create-deno-script',
			popover: {
				title: 'Create a Deno script',
				description:
					'Click here to create a Deno script. Deno is a secure runtime for JavaScript and TypeScript. You can use it to write scripts that can be executed in the background.',
				onNextClick: () => {
					clickButtonBySelector('#create-deno-script')

					setTimeout(() => {
						driver.moveNext()
					})
				}
			}
		},
		{
			element: '#app-editor-script-transformer',
			popover: {
				title: 'Script transformer',
				description:
					"Here you can add a script transformer. A script transformer is an optional frontend script that is executed right after the component's script whose purpose is to do lightweight transformation in the browser. It takes the previous computation's result as `result`"
			}
		}
	]}
/>
