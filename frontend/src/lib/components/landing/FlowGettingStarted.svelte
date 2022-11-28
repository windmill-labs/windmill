<script>
	import { goto } from '$app/navigation'
	import { faExternalLink, faFile, faPlus } from '@fortawesome/free-solid-svg-icons'
	import Button from '../common/button/Button.svelte'
	import HatIcon from '../icons/HatIcon.svelte'
</script>

<div class=" overflow-auto">
	<div class="mt-2 mb-4 text-sm text-gray-700 dark:text-gray-300">
		Flows allow you to streamline complex processes and operations by chaining simple steps
		together. Each Flow is composed of one or more steps.
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
