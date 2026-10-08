<script lang="ts">
	import ResourceEditor from '$lib/components/ResourceEditor.svelte'
	import { Button } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'

	// Mirror the React SDK's <ResourceEditor> surface: newResource toggles
	// between create/edit, resource_type seeds creation, path selects the
	// resource to edit. Remount on change via {#key}.
	let newResource = $state(true)
	let resource_type = $state('postgresql')
	let path = $state('')
	let hidePath = $state(false)

	let workspace = $state($workspaceStore ?? 'admins')

	// Test-dev routes don't go through the (logged) layout, so the workspace
	// store may be empty. Match what the React SDK does in main.tsx and set it
	// explicitly — ResourceEditor renders nothing until effectiveWorkspace
	// resolves.
	$effect(() => {
		if (workspace && $workspaceStore !== workspace) {
			workspaceStore.set(workspace)
		}
	})

	$effect(() => {
		if ($workspaceStore && !$userStore) {
			getUserExt($workspaceStore).then((u) => ($userStore = u))
		}
	})

	let lastChange = $state<{ path: string; args: any; description: string } | undefined>(undefined)

	let editor: ResourceEditor | undefined = $state()
	let canSave = $state(true)
</script>

<div class="p-4 flex flex-col gap-4">
	<div class="flex flex-wrap gap-4 items-end p-3 border rounded bg-surface-secondary">
		<label class="flex flex-col gap-1 text-xs">
			workspace
			<TextInput bind:value={workspace} inputProps={{ placeholder: 'admins' }} />
		</label>
		<Toggle bind:checked={newResource} options={{ right: 'newResource' }} />
		{#if newResource}
			<label class="flex flex-col gap-1 text-xs">
				resource_type
				<TextInput bind:value={resource_type} inputProps={{ placeholder: 'postgresql' }} />
			</label>
		{:else}
			<label class="flex flex-col gap-1 text-xs">
				path
				<TextInput bind:value={path} inputProps={{ placeholder: 'u/admin/my_resource' }} />
			</label>
		{/if}
		<Toggle bind:checked={hidePath} options={{ right: 'hidePath' }} />
		<Button disabled={!canSave} onclick={() => editor?.save()}>Save</Button>
	</div>

	<pre class="text-xs p-2 bg-surface-secondary rounded overflow-auto max-h-40"
		>workspace: {$workspaceStore ?? '(unset)'}
user: {$userStore?.username ?? '(unset)'}
onChange: {JSON.stringify(lastChange, null, 2)}</pre>

	{#if !$workspaceStore || !$userStore}
		<div class="text-xs text-orange-600">
			Waiting for workspace/user to load. If this never resolves, log into the app at /user/login
			first so the workspace cookie/store is set.
		</div>
	{:else}
		{#key `${newResource}|${resource_type}|${path}`}
			<ResourceEditor
				bind:this={editor}
				bind:canSave
				{hidePath}
				{...newResource ? { resource_type } : { path }}
				onChange={(e) => {
					console.log('onChange', e)
					lastChange = e
				}}
			/>
		{/key}
	{/if}
</div>
