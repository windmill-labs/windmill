<script lang="ts">
	import { driver, type Driver, type DriveStep } from 'driver.js'
	import { mount, onDestroy } from 'svelte'
	import TutorialControls from './TutorialControls.svelte'
	import TutorialInner from './TutorialInner.svelte'

	interface Props {
		/** Called once the tour ends, however it ended: last step, close button, or Escape. */
		onDestroyed?: () => void
		getSteps: (driver: Driver) => DriveStep[]
	}

	let { onDestroyed = undefined, getSteps }: Props = $props()

	let totalSteps = 0
	let tutorial: Driver | undefined = $state(undefined)

	// driver.js renders its popover as plain DOM, so the controls are mounted into it rather
	// than declared in markup — which is also why they are re-mounted on every step.
	function renderControls(activeIndex: number) {
		const popoverContent = document.querySelector('#driver-popover-content')
		popoverContent?.addEventListener('pointerdown', (event) => {
			event.stopPropagation()
		})

		const popoverDescription = document.querySelector('#driver-popover-description')
		if (!tutorial || !popoverDescription) {
			return
		}

		const controls = document.createElement('div')
		mount(TutorialControls, {
			target: controls,
			props: {
				activeIndex,
				totalSteps,
				// A step that defines `onNextClick` owns its own advance — that is how a step
				// that has to open something first waits for it before moving on.
				onNext: () => {
					const step = tutorial?.getActiveStep()
					if (!step) return
					const onNextClick = step.popover?.onNextClick
					if (onNextClick) {
						onNextClick(tutorial?.getActiveElement(), step, {
							config: tutorial!.getConfig(),
							state: tutorial!.getState(),
							driver: tutorial!,
							index: activeIndex
						})
					} else {
						tutorial?.moveNext()
					}
				},
				onPrevious: () => {
					const step = tutorial?.getActiveStep()
					if (!step) return
					const onPrevClick = step.popover?.onPrevClick
					if (onPrevClick) {
						onPrevClick(tutorial?.getActiveElement(), step, {
							config: tutorial!.getConfig(),
							state: tutorial!.getState(),
							driver: tutorial!,
							index: activeIndex
						})
					} else {
						tutorial?.movePrevious()
					}
				}
			}
		})
		popoverDescription.appendChild(controls)
	}

	export function runTutorial() {
		tutorial = driver({
			allowClose: true,
			disableActiveInteraction: true,
			showButtons: ['close'],
			showProgress: false,
			overlayColor: 'rgba(0, 0, 0, 0.8)',
			onPopoverRender: (_popover, { state }) => {
				renderControls(state.activeIndex ?? 0)
			},
			onDestroyed: () => {
				onDestroyed?.()
			}
		})

		const steps = getSteps(tutorial)
		totalSteps = steps.length
		tutorial.setSteps(steps)
		tutorial.drive()
	}

	// driver.js appends its overlay to the body, so leaving the page mid-tour would strand it
	// over whatever renders next. Destroying also runs `onDestroyed`, which is where the tour
	// is recorded as seen — so navigating away counts as having been shown it.
	onDestroy(() => tutorial?.destroy())
</script>

{#if tutorial}
	<TutorialInner />
{/if}
