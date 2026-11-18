<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
</script>

<Tooltip documentationLink="https://www.windmill.dev/docs/core_concepts/roles_and_permissions">
	<div class="flex flex-col gap-1">
		<span>Access comes from three places:</span>
		<span><b>Folder</b>: the members of the folder it lives in. Most access is set there.</span>
		<span><b>Owner</b>: the user or group its path starts with.</span>
		<span><b>Extra permission</b>: people given access to this item alone.</span>
		<span>Workspace admins can always see, run and edit it.</span>
		<span>Viewers can see and run it. Writers can also edit and deploy it.</span>
	</div>
</Tooltip>
