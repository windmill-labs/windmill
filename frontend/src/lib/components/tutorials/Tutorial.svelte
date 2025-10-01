<script lang="ts">
	import { driver, type Driver, type DriveStep } from 'driver.js'
	import { createEventDispatcher, mount } from 'svelte'
	import { updateProgress } from '$lib/tutorialUtils'
	import { ignoredTutorials } from './ignoredTutorials'
	import SkipTutorials from './SkipTutorials.svelte'
	import TutorialControls from './TutorialControls.svelte'
	import TutorialInner from './TutorialInner.svelte'
	import { isCurrentlyInTutorial } from '$lib/stores'

	export let index: number = 0
	export let name: string = 'action'
	export let tainted: boolean = false
	export let onDestroyed: (() => void) | undefined = undefined

	type Options = {
		indexToInsertAt?: number
		skipStepsCount?: number
	}

	export let getSteps: (driver: Driver, options?: Options | undefined) => DriveStep[] = () => []

	let totalSteps = 0
	let tutorial: Driver | undefined = undefined
	const dispatch = createEventDispatcher()

	// Render controls needs to be exposed so steps that have a custom render can call it
	export function renderControls({ config, state }) {
		const popoverContent = document.querySelector('#driver-popover-content')
		popoverContent?.addEventListener('pointerdown', (event) => {
			event.stopPropagation()
		})

		const popoverDescription = document.querySelector('#driver-popover-description')

		if (!tutorial) {
			return
		}

		if (state.activeIndex == 0) {
			const div = document.createElement('div')

			mount(SkipTutorials, {
				target: div,
				events: {
					skipAll: () => {
						dispatch('skipAll')
						tutorial?.destroy()
					},
					skipThis: () => {
						updateProgress(index)
						tutorial?.destroy()
					}
				}
			})

			if (popoverDescription) {
				popoverDescription.appendChild(div)
			}
		}

		const controls = document.createElement('div')

		mount(TutorialControls, {
			target: controls,
			props: {
				activeIndex: state.activeIndex,
				totalSteps
			},
			events: {
				next: () => {
					const step = tutorial?.getActiveStep()

					if (step) {
						if (tutorial?.getActiveStep()?.popover?.onNextClick) {
							const activeElement = tutorial?.getActiveElement()
							tutorial?.getActiveStep()?.popover?.onNextClick?.(activeElement, step, {
								config,
								state,
								driver: tutorial
							})
						} else {
							tutorial?.moveNext()
						}
					}
				},
				previous: () => {
					const step = tutorial?.getActiveStep()

					if (step) {
						if (tutorial?.getActiveStep()?.popover?.onPrevClick) {
							const activeElement = tutorial?.getActiveElement()
							tutorial?.getActiveStep()?.popover?.onPrevClick?.(activeElement, step, {
								config,
								state,
								driver: tutorial
							})
						} else {
							tutorial?.movePrevious()
						}
					}
				}
			}
		})

		if (popoverDescription) {
			popoverDescription.appendChild(controls)
		}
	}

	export const runTutorial = (options?: Options | undefined) => {
		if (tainted) {
			dispatch('error', { detail: name })
			return
		}
		isCurrentlyInTutorial.val = true

		tutorial = driver({
			allowClose: true,
			disableActiveInteraction: true,
			showButtons: ['close'],
			showProgress: false,
			overlayColor: 'rgba(0, 0, 0, 0.8)',
			onPopoverRender: (popover, { config, state }) => {
				renderControls({ config, state })
			},
			onDestroyed: () => {
				onDestroyed?.()
				if (!tutorial?.hasNextStep()) {
					$ignoredTutorials = Array.from(new Set([...$ignoredTutorials, index]))
				}
				isCurrentlyInTutorial.val = false
			}
		})

		const steps = getSteps(tutorial, options)

		totalSteps = steps.length

		tutorial.setSteps(steps)
		tutorial.drive()
	}
</script>

{#if tutorial}
	<TutorialInner />
{/if}
