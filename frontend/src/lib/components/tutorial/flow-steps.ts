import type Shepherd from 'shepherd.js'

export default (tour: Shepherd.Tour) => {
	const steps: object[] | Shepherd.Step[] = [
		{
			id: 'welcome',
			text: `<div class="text-xl font-bold mb-6">Windmill Flow Editor</div>
			<p class="mb-6">Build complex workflows from your scripts, configure them and get a clear visualization.</p>`,
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
			id: 'graph',
			text: `<div class="text-xl font-bold mb-2">Flow graph</div>
			<p class="mb-2">Know how your flow is structured at a glimpse.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#flow-tutorial-1', on: 'right-start' },
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
			id: 'modules',
			text: `<div class="text-xl font-bold mb-2">Insert modules</div>
			<p class="mb-2">Start building by adding modules. A module can be a simple script, a loop or a branching for example.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#flow-tutorial-2 button', on: 'right-start' },
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
			id: 'settings',
			text: `<div class="text-xl font-bold mb-2">Settings</div>
			<p class="mb-2">Configure schedules, authorizations, sharing parameters and many more.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#flow-tutorial-3', on: 'bottom-start' },
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
			text: `<div class="text-xl font-bold mb-2">Test flow</div>
			<p class="mb-2">Time to try your work! Test the flow and iterate as much as you want.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#flow-tutorial-4', on: 'left-start' },
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
