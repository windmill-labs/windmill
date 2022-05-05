<script lang="ts">
	import { type Resource, ResourceService } from '../../gen';
	import { workspaceStore } from '../../stores';

	let resources: Resource[] = [];

	export let value: string | undefined;

	export let resourceType: string | undefined;

	async function loadResources(resourceType: string | undefined) {
		resources = await ResourceService.listResource({ workspace: $workspaceStore!, resourceType });
	}

	$: {
		if ($workspaceStore) {
			loadResources(resourceType);
		}
	}
</script>

<select class="mt-1" bind:value placeholder="Pick a resource {resourceType}">
	<option value={undefined} />
	{#each resources as r}
		<option value={r.path}>{r.path}{r.description ? ' | ' + r.description : ''}</option>
	{/each}
</select>
