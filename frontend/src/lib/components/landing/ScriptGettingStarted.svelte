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
		Create a new script or find inspiration on the Hub!
	</div>
	<div class="inline-flex flex-wrap gap-y-2">
		<Button
			href="/scripts/add"
			color="dark"
			size="xs"
			btnClasses="mr-2"
			startIcon={{ icon: faPlus }}
		>
			Create script
		</Button>
		<Button
			on:click={() => goto('https://docs.windmill.dev/docs/getting_started/scripts')}
			color="blue"
			size="xs"
			btnClasses="mr-2"
			startIcon={{ icon: faFile }}
		>
			Script documentation
		</Button>
		<Button
			on:click={() => goto('https://hub.windmill.dev/')}
			color="light"
			variant="border"
			size="xs"
			startIcon={{ icon: faExternalLink }}
			target="_blank"
		>
			Explore community scripts on WindmillHub
		</Button>
	</div>
</div>
