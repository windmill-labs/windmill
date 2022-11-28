<script>
	import { goto } from '$app/navigation'
	import { faLink } from '@fortawesome/free-solid-svg-icons'
	import Button from '../common/button/Button.svelte'
	import HatIcon from '../icons/HatIcon.svelte'
</script>

<div class="fle flex-row gap-2">
	<div class="mt-2 mb-4 text-sm text-gray-700 dark:text-gray-300">
		Connect to apps like Slack, Google Drive or Airtable using OAuth.
	</div>
	<Button href="/resources" color="dark" startIcon={{ icon: faLink }} size="xs">
		Connect an API
	</Button>
</div>
