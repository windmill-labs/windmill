import type Shepherd from 'shepherd.js'

export default (tour: Shepherd.Tour) => {
	const steps: object[] | Shepherd.Step[] = [
		{
			id: 'welcome',
			text: `<div class="text-xl font-bold mb-6">Windmill App Editor</div>
			<p class="mb-6">Build your own UI quick and easy with drag-and-drop components. Connect your scripts and data to them and deploy a fully functioning application.</p>`,
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
			id: 'components',
			text: `<div class="text-xl font-bold mb-2">Components</div>
			<p class="mb-2">Click to add and move them around by dragging. Configure the inputs, settings and styling, even with Tailwind classes.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-1', on: 'left-start' },
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
			id: 'outputs',
			text: `<div class="text-xl font-bold mb-2">Outputs</div>
			<p class="mb-2">Each component has their outputs, which can be used to easily hook into the app state.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-2', on: 'right-start' },
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
			text: `<div class="text-xl font-bold mb-2">Canvas</div>
			<p class="mb-2">It allows you to position, scale and group the components.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-3', on: 'bottom' },
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
			text: `<div class="text-xl font-bold mb-2">Runnable editor</div>
			<p class="mb-2">Here you can create, edit and manage the scripts of your app.</p>`,
			cancelIcon: {
				enabled: true
			},
			attachTo: { element: '#app-tutorial-4', on: 'top-start' },
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
