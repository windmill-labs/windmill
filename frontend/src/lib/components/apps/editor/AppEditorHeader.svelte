<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { faExternalLink } from '@fortawesome/free-solid-svg-icons'

	export let title: string
</script>

<div class="border-b h-12 flex flex-row justify-between py-2 px-4 items-center">
	<span class="text-sm">{title}</span>
	<div class="flex flex-row gap-2">
		<Badge color="dark-indigo">Draft</Badge>
		<Button size="sm" variant="border" startIcon={{ icon: faExternalLink }}>Publish</Button>
		<Button size="sm">Save</Button>
	</div>
</div>
