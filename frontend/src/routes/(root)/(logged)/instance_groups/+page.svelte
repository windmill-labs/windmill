<script lang="ts">
	import { run } from 'svelte/legacy'

	import type { InstanceGroup } from '$lib/gen'
	import { GroupService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import InstanceGroupEditor from '$lib/components/InstanceGroupEditor.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { Plus } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	let newGroupName: string = $state('')
	let instanceGroups: InstanceGroup[] | undefined = $state(undefined)
	let groupDrawer: Drawer | undefined = $state()

	async function loadInstanceGroups(): Promise<void> {
		try {
			instanceGroups = await GroupService.listInstanceGroups()
		} catch (e) {
			instanceGroups = undefined
		}
	}

	async function addInstanceGroup() {
		await GroupService.createInstanceGroup({
			requestBody: { name: newGroupName }
		})
		loadInstanceGroups()
		editGroupName = newGroupName
		groupDrawer?.openDrawer()
	}

	run(() => {
		loadInstanceGroups()
	})

	let editGroupName: string = $state('')
</script>

<Drawer bind:this={groupDrawer}>
	<DrawerContent title="Instance Group {editGroupName}" on:close={groupDrawer.closeDrawer}>
		<InstanceGroupEditor on:update={loadInstanceGroups} name={editGroupName} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader title="Instance Groups">
		<div class="flex flex-row">
			<div>
				<Popover
					floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
					containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
				>
					{#snippet trigger()}
						<Button size="md" startIcon={{ icon: Plus }} nonCaptureEvent>New&nbsp;group</Button>
					{/snippet}
					{#snippet content({ close })}
						<div class="flex-col flex gap-2 p-4">
							<input class="mr-2" placeholder="New instance group name" bind:value={newGroupName} />
							<Button
								size="md"
								startIcon={{ icon: Plus }}
								disabled={!newGroupName}
								on:click={() => {
									addInstanceGroup()
									close()
								}}
							>
								Create
							</Button>
						</div>
					{/snippet}
				</Popover>
			</div>
		</div>
	</PageHeader>

	{#if instanceGroups && instanceGroups.length > 0}
		<div class="relative mb-20 pt-8">
			<TableCustom>
				<!-- @migration-task: migrate this slot by hand, `header-row` is an invalid identifier -->
				<tr slot="header-row">
					<th>Name</th>
					<th>Summary</th>
					<th>Members</th>
					<th></th>
				</tr>
				{#snippet body()}
					<tbody>
						{#each instanceGroups as { name, summary, emails }}
							<tr>
								<td>
									<a
										href="#{name}"
										onclick={() => {
											editGroupName = name
											groupDrawer?.openDrawer()
										}}
										>{name}
									</a>
								</td>
								<td>
									{summary ? summary.slice(0, 50) + (summary.length > 50 ? '...' : '') : '-'}
								</td>
								<td>{emails?.length ?? 0} members</td>
								<td
									><Button
										variant="default"
										destructive
										on:click={async () => {
											await GroupService.deleteInstanceGroup({ name })
											sendUserToast(`Instance group ${name} deleted`)
											loadInstanceGroups()
										}}>Delete</Button
									></td
								>
							</tr>
						{/each}
					</tbody>
				{/snippet}
			</TableCustom>
		</div>
	{/if}
</CenteredPage>

<style>
</style>
