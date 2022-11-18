<script>
	import { goto } from '$app/navigation'
	import { faExternalLink, faFile, faPlus } from '@fortawesome/free-solid-svg-icons'
	import Button from '../common/button/Button.svelte'
	import HatIcon from '../icons/HatIcon.svelte'
</script>

<div class="p-4 border border-gray-300 rounded-md bg-gray-50 overflow-auto">
	<div class="flex items-center">
		<h3 class="inline-flex items-center font-semibold">
			<HatIcon class="w-6 h-6 mr-2" />
			Getting started
		</h3>
	</div>
	<div class="mt-2 mb-4 text-sm text-gray-700 ">
		Flows allow you to streamline complex processes and operations by chaining simple steps
		together. Each Flow is composed of one or more steps. Create a new flow or find inspiration on
		the Hub!
	</div>
	<div class="flex flex-row flex-wrap gap-y-2">
		<Button href="/flows/add" color="dark" size="xs" btnClasses="mr-2" startIcon={{ icon: faPlus }}>
			Create flow
		</Button>
		<Button
			on:click={() => goto('https://docs.windmill.dev/docs/getting_started/flows')}
			color="blue"
			size="xs"
			btnClasses="mr-2"
			startIcon={{ icon: faFile }}
		>
			Flow documentation
		</Button>
		<Button
			on:click={() => goto('https://hub.windmill.dev/')}
			color="light"
			variant="border"
			size="xs"
			startIcon={{ icon: faExternalLink }}
			target="_blank"
		>
			Explore community flows on WindmillHub
		</Button>
	</div>
</div>
