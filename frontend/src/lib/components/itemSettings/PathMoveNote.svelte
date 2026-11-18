<script lang="ts">
	import { Alert } from '$lib/components/common'
	import { movesToAnotherOwner, ownerLabel } from './inheritedAccess'

	interface Props {
		kind: 'flow' | 'script' | 'app'
		/** Where the deployed item is. Undefined before its first deploy, which moves nothing. */
		deployedPath: string | undefined
		/** The path the next deploy gives it. */
		path: string
	}

	let { kind, deployedPath, path }: Props = $props()
</script>

{#if deployedPath && path && path !== deployedPath}
	{#if movesToAnotherOwner(deployedPath, path)}
		<Alert
			type="warning"
			size="xs"
			title="Moving it from {ownerLabel(deployedPath)} to {ownerLabel(path)} changes who has access"
			class="mt-2"
		>
			Deploying shows who loses access and asks you to confirm.
		</Alert>
	{:else}
		<Alert
			type="info"
			size="xs"
			title="Deploy the {kind} to make the path change effective."
			class="mt-2"
		/>
	{/if}
{/if}
