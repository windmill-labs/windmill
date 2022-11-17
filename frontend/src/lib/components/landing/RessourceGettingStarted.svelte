<script>
	import { goto } from '$app/navigation'
	import { faLink } from '@fortawesome/free-solid-svg-icons'
	import Button from '../common/button/Button.svelte'
	import HatIcon from '../icons/HatIcon.svelte'
</script>

<div class="p-4 border border-gray-300 rounded-md bg-gray-50 dark:border-gray-600 dark:bg-gray-700">
	<div class="flex items-center">
		<h3 class="inline-flex items-center font-semibold">
			<HatIcon class="w-6 h-6 mr-2" />
			Getting started
		</h3>
	</div>
	<div class="mt-2 mb-4 text-sm text-gray-700 dark:text-gray-300">
		Connect to apps like Slack, Google Drive or Airtable using OAuth.
	</div>
	<Button href="/resources" color="dark" startIcon={{ icon: faLink }} size="xs">
		Connect an API
	</Button>
</div>
