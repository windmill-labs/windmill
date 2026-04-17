<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { resource } from 'runed'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonMore from './common/toggleButton-v2/ToggleButtonMore.svelte'

	interface Props {
		kind: 'resource' | 'variable'
		workspaceId: string
		initialPath: string
		selected: string | undefined
	}

	let { kind, workspaceId, initialPath, selected = $bindable() }: Props = $props()

	let versionsResource = resource(
		[() => ({ path: initialPath, ws: workspaceId, kind })],
		async ([{ path, ws, kind }]) => {
			return await WorkspaceService.listWsSpecificVersions({ workspace: ws, kind, path })
		}
	)

	let versions = $derived([
		workspaceId,
		...(versionsResource.current?.filter((v) => v != workspaceId) ?? [])
	])
	let regular = $derived(versions.filter((v) => !v.startsWith('wm-fork-') || v === workspaceId))
	let forks = $derived(versions.filter((v) => v.startsWith('wm-fork-') && v !== workspaceId))
</script>

{#if versions.length > 1}
	<ToggleButtonGroup bind:selected>
		{#snippet children({ item })}
			{#each regular as v (v)}
				<ToggleButton value={v} label={v} {item} />
			{/each}
			{#if forks.length > 0}
				<ToggleButtonMore
					btnText="Forks"
					togglableItems={forks.map((v) => ({ label: v, value: v }))}
					{item}
					bind:selected
				/>
			{/if}
		{/snippet}
	</ToggleButtonGroup>
{/if}
