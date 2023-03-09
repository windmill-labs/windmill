import type Shepherd from 'shepherd.js'

export default (tour: Shepherd.Tour) => {
	const steps: object[] | Shepherd.Step[] = [
		{
			id: 'welcome',
			text: `<div class="text-xl font-bold mb-6">Welcome to Windmill Cloud App!</div>
			<p class="mb-6">Thank your for your interest in Windmill. Let us guide you through the home screen in strictly less than a minute.</p>`,
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
			id: 'workspace',
			text: `<div class="text-xl font-bold mb-2">Workspace</div>
			<p>You can see all the scripts, flows and apps here.</p>
			<p class="mb-2">Use the search and filters to find just what you are looking for.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#welcome-tutorial-1', on: 'bottom-start' },
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
			id: 'create',
			text: `<div class="text-xl font-bold mb-2">Create</div>
			<p class="mb-2">Build your own tools in minutes.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#welcome-tutorial-2', on: 'bottom-end' },
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
			id: 'inspiration',
			text: `<div class="text-xl font-bold mb-2">Take inspiration</div>
			<p class="mb-2">Explore templates built by the community and import them directly from <a href="https://hub.windmill.dev" target="_blank">Windmill Hub</a>.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#welcome-tutorial-3', on: 'bottom-start' },
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
			id: 'monitor',
			text: `<div class="text-xl font-bold mb-2">Monitor</div>
			<p class="mb-2">Keep an eye an all past and scheduled executions of scripts and flows.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#welcome-tutorial-4-wrapper #welcome-tutorial-4', on: 'right-start' },
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
