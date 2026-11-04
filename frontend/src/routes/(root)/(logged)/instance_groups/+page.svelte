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
	import { page } from '$app/state'
	import { isNewTabClick } from '$lib/navigation'

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

	// Deep link: #<name> opens that group's drawer. Row links pre-set handledName: their onclick
	// already opens the drawer. Names are compared decoded since the browser percent-encodes the
	// hash.
	let handledName = ''
	$effect(() => {
		const hash = page.url.hash
		if (hash.length <= 1) {
			handledName = ''
			return
		}
		let name: string
		try {
			name = decodeURIComponent(hash.slice(1))
		} catch {
			return
		}
		if (name === handledName || !instanceGroups || !groupDrawer) return
		if (instanceGroups.some((g) => g.name === name)) {
			handledName = name
			editGroupName = name
			groupDrawer.openDrawer()
		}
	})
</script>

<Drawer bind:this={groupDrawer}>
	<DrawerContent title="Instance Group {editGroupName}" on:close={groupDrawer?.closeDrawer}>
		<InstanceGroupEditor on:update={loadInstanceGroups} name={editGroupName} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader title="Instance Groups">
		<div class="flex flex-row">
			<div>
				<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
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
				{#snippet headerRow()}
					<tr>
						<th>Name</th>
						<th>Summary</th>
						<th>Members</th>
						<th>Instance Role</th>
						<th></th>
					</tr>
				{/snippet}
				{#snippet body()}
					<tbody>
						{#each instanceGroups as { name, summary, emails, instance_role }}
							<tr>
								<td>
									<a
										href="#{encodeURIComponent(name)}"
										onclick={(e) => {
											if (isNewTabClick(e)) return
											handledName = name
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
								<td>{instance_role ?? '-'}</td>
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
