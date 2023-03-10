import type Shepherd from 'shepherd.js'

export default (tour: Shepherd.Tour) => {
	const steps: object[] | Shepherd.Step[] = [
		{
			id: 'welcome',
			text: `<div class="text-xl font-bold mb-6">Windmill Script Builder</div>
			<p class="mb-6">Build scripts with out own web editor, then use them anywhere in your flows, apps or as standalone computations with their auto-generated UI.</p>`,
			buttons: [
				{
					text: 'See by myself',
					action: tour.cancel,
					secondary: true,
					classes: 'special'
				},
				{
					text: 'Give me an overview',
					action: tour.next,
					classes: 'special'
				}
			]
		},
		{
			id: 'editor',
			text: `<div class="text-xl font-bold mb-2">Web editor</div>
			<p class="mb-2">Code directly from the web editor.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-1', on: 'bottom-start' },
			highlightClass: 'shepherd-highlighted-element',
			scrollTo: false,
			buttons: [
				{
					text: 'Back',
					action: tour.back,
					secondary: true
				},
				{
					text: 'Next',
					action: tour.next
				}
			]
		},
		{
			id: 'preview',
			text: `<div class="text-xl font-bold mb-2">UI preview</div>
			<p class="mb-2">Preview the user interface generated from the signature of the script.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-2', on: 'left-start' },
			highlightClass: 'shepherd-highlighted-element',
			scrollTo: false,
			buttons: [
				{
					text: 'Back',
					action: tour.back,
					secondary: true
				},
				{
					text: 'Next',
					action: tour.next
				}
			]
		},
		{
			id: 'parameters',
			text: `<div class="text-xl font-bold mb-2">Parameters</div>
			<p class="mb-2">Add variables, resources and configure your script.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-3', on: 'bottom-start' },
			highlightClass: 'shepherd-highlighted-element',
			scrollTo: false,
			buttons: [
				{
					text: 'Back',
					action: tour.back,
					secondary: true
				},
				{
					text: 'Next',
					action: tour.next
				}
			]
		},
		{
			id: 'test',
			text: `<div class="text-xl font-bold mb-2">Test script</div>
			<p class="mb-2">Time to try your work! Test the script and iterate as much as you want.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-4', on: 'left-start' },
			highlightClass: 'shepherd-highlighted-element',
			scrollTo: false,
			buttons: [
				{
					text: 'Back',
					action: tour.back,
					secondary: true
				},
				{
					text: 'Finish',
					action: tour.complete
				}
			]
		}
	]
	return steps
}
