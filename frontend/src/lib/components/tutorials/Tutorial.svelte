<script lang="ts">
	import { driver, type Driver, type DriveStep } from 'driver.js'
	import 'driver.js/dist/driver.css'
	import { createEventDispatcher } from 'svelte'
	import { updateProgress } from '$lib/tutorialUtils'
	import { ignoredTutorials } from './ignoredTutorials'
	import SkipTutorials from './SkipTutorials.svelte'
	import TutorialControls from './TutorialControls.svelte'

	export let index: number = 0
	export let name: string = 'action'
	export let tainted: boolean = false

	type Options = {
		indexToInsertAt?: number
		skipStepsCount?: number
	}

	export let getSteps: (driver: Driver, options?: Options | undefined) => DriveStep[] = () => []

	let totalSteps = 0

	const dispatch = createEventDispatcher()

	export const runTutorial = (options?: Options | undefined) => {
		if (tainted) {
			dispatch('error', { detail: name })
			return
		}

		const tutorial = driver({
			allowClose: true,
			disableActiveInteraction: true,
			showButtons: ['close'],
			showProgress: false,
			overlayColor: 'rgba(0, 0, 0, 0.8)',
			onPopoverRender: (popover, { config, state }) => {
				const popoverDescription = document.querySelector('#driver-popover-description')

				if (state.activeIndex == 0) {
					const div = document.createElement('div')

					const skipTutorials = new SkipTutorials({
						target: div
					})

					skipTutorials.$on('skipAll', () => {
						dispatch('skipAll')
						tutorial.destroy()
					})

					skipTutorials.$on('skipThis', () => {
						updateProgress(index)
						tutorial.destroy()
					})

					if (popoverDescription) {
						popoverDescription.appendChild(div)
					}
				}

				const controls = document.createElement('div')

				console.log(state)

				const tutorialControls = new TutorialControls({
					target: controls,
					props: {
						activeIndex: state.activeIndex,
						totalSteps
					}
				})

				tutorialControls.$on('next', () => {
					tutorial.moveNext()
				})

				tutorialControls.$on('previous', () => {
					tutorial.movePrevious()
				})

				if (popoverDescription) {
					popoverDescription.appendChild(controls)
				}
			},
			onDestroyed: () => {
				if (!tutorial.hasNextStep()) {
					$ignoredTutorials = Array.from(new Set([...$ignoredTutorials, index]))
				}
			}
		})

		const steps = getSteps(tutorial, options)

		totalSteps = steps.length

		tutorial.setSteps(steps)
		tutorial.drive()
	}
</script>

<style>
	:global(.driver-popover) {
		padding: 32px;
	}
	:global(.driver-popover-title) {
		font-size: 1.2rem !important;
		line-height: 2 !important;
	}
</style>
