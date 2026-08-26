<script lang="ts">
	import { Drawer } from '$lib/components/common'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { Label, LabelColor } from '$lib/gen'
	import LabelBadge from './LabelBadge.svelte'
	import LabelColorPicker from './LabelColorPicker.svelte'
	import { loadLabels, setLabelColor } from './labelStore'

	interface Props {
		workspace: string | undefined
	}

	let { workspace }: Props = $props()

	let drawer: Drawer | undefined = $state()
	let labels: Label[] | undefined = $state()

	export async function open() {
		labels = undefined
		drawer?.openDrawer()
		await refresh()
	}

	async function refresh() {
		if (!workspace) return
		labels = await loadLabels(workspace, true)
	}

	async function pick(name: string, color: LabelColor | undefined) {
		if (!workspace) return
		try {
			labels = await setLabelColor(workspace, name, color)
		} catch (err) {
			sendUserToast(`Could not update label ${name}: ${err}`, true)
		}
	}
</script>

<Drawer bind:this={drawer} size="32rem">
	<DrawerContent title="Labels" on:close={drawer?.closeDrawer}>
		{#if labels == undefined}
			<Skeleton layout={[[2], 1, [2], 1, [2]]} />
		{:else if labels.length === 0}
			<p class="text-sm text-secondary">
				No labels yet. Labels appear here once you add one to a script, flow, app, resource,
				variable, schedule or folder.
			</p>
		{:else}
			<p class="text-xs text-secondary mb-4">
				A color applies to the label everywhere it is used in this workspace.
			</p>
			<div class="flex flex-col divide-y">
				{#each labels as label (label.name)}
					<div class="flex items-center py-2">
						<LabelColorPicker color={label.color} onSelect={(c) => pick(label.name, c)}>
							{#snippet anchor()}
								<LabelBadge
									label={label.name}
									{workspace}
									title="Pick a color for {label.name}"
								/>
							{/snippet}
						</LabelColorPicker>
					</div>
				{/each}
			</div>
		{/if}
	</DrawerContent>
</Drawer>
