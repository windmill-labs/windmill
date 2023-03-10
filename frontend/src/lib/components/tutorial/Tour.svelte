<script lang="ts">
	import Shepherd from 'shepherd.js'
	import { steps, tourStore, type TutorialName } from './'

	export let tutorial: TutorialName
	/**
	 * If the index of a step is added to a tour, the custom footer will not be added
	 * (ie. the steps won't be shown and the buttons won't be positioned)
	 */
	const noPageNumber: Partial<Record<TutorialName, number[]>> = {
		welcome: [0]
	}
	let tour: Shepherd.Tour | undefined

	$: $tourStore.includes(tutorial) && initiate()

	function initiate() {
		if (!tourStore.isTourActive(tutorial)) {
			return
		}
		tour?.cancel()
		tour = new Shepherd.Tour({
			tourName: tutorial,
			useModalOverlay: true,
			defaultStepOptions: {
				classes: 'max-w-[460px] shadow-xl bg-white border rounded px-6 py-4 z-[1100]',
				scrollTo: true
			}
		})
		tour.addSteps(steps[tutorial](tour))
		tour.steps.forEach((step, i) => {
			if (noPageNumber[tutorial]?.includes(i)) {
				return
			}
			step.on('show', () => {
				const currentStep = tour?.getCurrentStep()
				const currentStepElement = currentStep?.getElement()
				const header = currentStepElement?.querySelector('.shepherd-header')
				if (currentStep && currentStepElement && header) {
					const progress = document.createElement('span')
					progress.className = 'shepherd-progress text-sm text-gray-500 pr-2'
					progress.innerText = `${(tour?.steps.indexOf(currentStep) ?? 0) + 1}/${
						tour?.steps.length
					}`
					header.prepend(progress)
				}
			})
		})
		;['complete', 'cancel'].forEach((event) => {
			tour?.on(event, () => {
				tourStore.markTourAsDone(tutorial)
			})
		})
		tour.start()
	}
</script>

<style global>
	.shepherd-footer {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: 0.5rem;
	}

	.shepherd-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 0.5rem;
	}

	.shepherd-button {
		padding: 4px 12px;
		border-radius: 4px;
		transition-duration: 200ms;
	}

	/* Simple primary button */
	.shepherd-button:not(.shepherd-button-secondary.special) {
		color: #3b82f6;
	}

	.shepherd-button:not(.shepherd-button-secondary.special):hover,
	.shepherd-button:not(.shepherd-button-secondary.special):focus {
		background-color: #eff6ff;
	}

	/* Simple secondary button */
	.shepherd-button.shepherd-button-secondary:not(.special) {
		color: #6b7280;
	}

	.shepherd-button.shepherd-button-secondary:not(.special):hover,
	.shepherd-button.shepherd-button-secondary:not(.special):focus {
		background-color: #f3f4f6;
	}

	/* Special primary button */
	.shepherd-button.special:not(.shepherd-button-secondary) {
		background-color: #3b82f6;
		color: #fff;
	}

	.shepherd-button.special:not(.shepherd-button-secondary):hover,
	.shepherd-button.special:not(.shepherd-button-secondary):focus {
		background-color: #2563eb;
	}

	/* Special secondary button */
	.shepherd-button.special.shepherd-button-secondary {
		background-color: #eff6ff;
		color: #3b82f6;
	}

	.shepherd-button.special.shepherd-button-secondary:hover,
	.shepherd-button.special.shepherd-button-secondary:focus {
		background-color: #dbeafe;
	}

	.shepherd-cancel-icon {
		display: flex;
		justify-content: center;
		align-items: center;
		width: 32px;
		height: 32px;
		border-radius: 16px;
		background-color: #fff;
		transition-duration: 200ms;
	}

	.shepherd-cancel-icon:hover,
	.shepherd-cancel-icon:focus {
		background-color: #ededed;
	}

	.shepherd-highlighted-element {
		box-shadow: 0 0 0 6px #93c4fdbb;
		background-color: #93c4fd3a;
		position: relative;
		z-index: 1090;
	}
</style>
