<script lang="ts">
	import Select from '$lib/components/select/Select.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	type Props = {
		username: string
		targetKind: 'user' | 'folder'
		selectedUser: string | undefined
		selectedFolder: string | undefined
		selectedOperator: string | undefined
		users: Array<{ label: string; value: string }>
		folders: Array<{ label: string; value: string }>
		showTargetSelector?: boolean
		size?: 'sm' | 'md'
	}

	let {
		username,
		targetKind = $bindable(),
		selectedUser = $bindable(),
		selectedFolder = $bindable(),
		selectedOperator = $bindable(),
		users,
		folders,
		showTargetSelector = true,
		size = 'md'
	}: Props = $props()

	$effect(() => {
		if (targetKind === 'user' && selectedUser && !selectedOperator) {
			selectedOperator = selectedUser
		}
	})
</script>

{#if showTargetSelector}
	<div class={size === 'sm' ? 'mb-2' : ''}>
		<span
			class={size === 'sm'
				? 'text-xs font-medium text-secondary block mb-1'
				: 'text-sm font-medium text-primary block mb-1.5'}>Move u/{username}/* items to</span
		>
		<ToggleButtonGroup
			selected={targetKind}
			onSelected={(value) => {
				targetKind = value
			}}
			class={size === 'sm' ? 'mb-1.5' : 'mb-2'}
		>
			{#snippet children({ item })}
				<ToggleButton value="user" label="User" small {item} />
				<ToggleButton value="folder" label="Folder" small {item} />
			{/snippet}
		</ToggleButtonGroup>
		{#if targetKind === 'user'}
			<Select items={users} bind:value={selectedUser} placeholder="Select a user..." {size} />
		{:else}
			<Select items={folders} bind:value={selectedFolder} placeholder="Select a folder..." {size} />
		{/if}
	</div>
{/if}

<div>
	<span
		class={size === 'sm'
			? 'text-xs font-medium text-secondary block mb-0.5'
			: 'text-sm font-medium text-primary block mb-1.5'}
		>Update triggers/runnables permissions to</span
	>
	<Select items={users} bind:value={selectedOperator} placeholder="Select a user..." {size} />
</div>
