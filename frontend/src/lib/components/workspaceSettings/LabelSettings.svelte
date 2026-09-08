<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import LabelBadge from '$lib/components/labels/LabelBadge.svelte'
	import LabelEditor from '$lib/components/labels/LabelEditor.svelte'
	import { loadLabels } from '$lib/components/labels/labelStore'
	import type { Label } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Pen, Plus } from 'lucide-svelte'

	let labels: Label[] | undefined = $state()
	let labelEditor: LabelEditor | undefined = $state()

	$effect(() => {
		const workspace = $workspaceStore
		if (workspace) {
			refresh(workspace)
		}
	})

	async function refresh(workspace: string) {
		labels = await loadLabels(workspace, true)
	}
</script>

<LabelEditor
	bind:this={labelEditor}
	workspace={$workspaceStore}
	onSave={() => $workspaceStore && refresh($workspaceStore)}
/>

<SettingsPageHeader
	title="Labels"
	description="Labels group scripts, flows, apps, resources, variables, schedules and folders. A color applies to the label everywhere it is used in this workspace."
>
	{#snippet actions()}
		<Button
			variant="accent"
			unifiedSize="sm"
			startIcon={{ icon: Plus }}
			onClick={() => labelEditor?.initNew()}
		>
			New label
		</Button>
	{/snippet}
</SettingsPageHeader>

{#if labels == undefined}
	<Skeleton layout={[[2], 1, [2], 1, [2]]} />
{:else if labels.length === 0}
	<p class="text-sm text-secondary">
		No labels yet. Create one here, or add one to a script, flow, app, resource, variable, schedule
		or folder.
	</p>
{:else}
	<div class="flex flex-col divide-y border-t border-b">
		{#each labels as label (label.name)}
			<div class="flex flex-row items-center justify-between gap-2 py-2">
				<LabelBadge label={label.name} workspace={$workspaceStore} />
				<Button
					variant="subtle"
					unifiedSize="xs"
					startIcon={{ icon: Pen }}
					iconOnly
					aria-label="Edit label {label.name}"
					onClick={() => labelEditor?.initEdit(label)}
				/>
			</div>
		{/each}
	</div>
{/if}
