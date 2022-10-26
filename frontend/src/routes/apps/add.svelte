<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Path from '$lib/components/Path.svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	/**
	 * Resource type: label -> apps
	 * Store JSON d
	 */

	let path: string = ''
	let initialPath = ''
	let pathError = ''

	function createApp() {
		const body = {}
		ResourceService.createResource({
			workspace: $workspaceStore!,
			requestBody: {
				resource_type: 'apps',
				value: {},
				path
			}
		})
	}
</script>

<Path bind:error={pathError} bind:path {initialPath} namePlaceholder="my_app" kind="app">
	<div slot="ownerToolkit">
		Resource permissions depend on their path. Select the group <span class="font-mono">all</span>
		to share it, and <span class="font-mono">user</span> to keep it private.
		<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
	</div>
</Path>

<Button startIcon={{ icon: faPlus }}>Add app</Button>
