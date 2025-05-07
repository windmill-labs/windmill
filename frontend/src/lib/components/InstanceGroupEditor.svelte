<script lang="ts">
	import { GroupService, type InstanceGroup } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import autosize from '$lib/autosize'
	import { Button } from './common'
	import Skeleton from './common/skeleton/Skeleton.svelte'
	import TableCustom from './TableCustom.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'

	export let name: string

	let email = ''
	let instance_group: InstanceGroup | undefined
	let members: { member_email: string }[] | undefined = undefined

	const dispatch = createEventDispatcher()

	$: {
		load()
	}

	async function load() {
		return Promise.all([loadInstanceGroup()])
	}

	async function loadInstanceGroup(): Promise<void> {
		instance_group = await GroupService.getInstanceGroup({ name })
		members = instance_group?.emails
			? instance_group.emails.map((x) => {
					return { member_email: x }
			  })
			: []
	}
</script>

<div class="flex flex-col gap-6">
	<h1>{name}</h1>
	{#if instance_group !== undefined}
		<div class="flex flex-col gap-1">
			<textarea
				rows="2"
				use:autosize
				bind:value={instance_group.summary}
				placeholder="Summary of the group"
			></textarea>
			<div class="flex justify-end">
				<Button
					size="xs"
					on:click={async () => {
						await GroupService.updateInstanceGroup({
							name,
							requestBody: { new_summary: instance_group?.summary ?? '' }
						})
						dispatch('update')
						sendUserToast('New summary saved')
					}}>Save Summary</Button
				>
			</div>
		</div>
		<h2
			>Members ({#if members?.length != undefined}{members?.length ?? 0}{:else}<Loader2
					class="animate-spin"
					size={10}
				/>{/if})</h2
		>
		<div class="flex items-start">
			<input type="text" placeholder="john@acme.com" bind:value={email} />
			<Button
				variant="contained"
				color="blue"
				size="sm"
				btnClasses="!ml-4"
				on:click={async () => {
					await GroupService.addUserToInstanceGroup({
						name,
						requestBody: { email: email }
					})
					dispatch('update')
					sendUserToast('User added')
					loadInstanceGroup()
				}}
			>
				Add member
			</Button>
		</div>
		{#if members}
			<TableCustom>
				<tr slot="header-row">
					<th>user</th>
					<th></th>
				</tr>
				<tbody slot="body">
					{#each members as { member_email }}<tr>
							<td>{member_email}</td>
							<td>
								<button
									class="ml-2 text-red-500"
									on:click={async () => {
										await GroupService.removeUserFromInstanceGroup({
											name,
											requestBody: { email: member_email }
										})
										dispatch('update')
										sendUserToast('User removed')
										loadInstanceGroup()
									}}>remove</button
								>
							</td>
						</tr>{/each}
				</tbody>
			</TableCustom>
		{:else}
			<div class="flex flex-col">
				{#each new Array(6) as _}
					<Skeleton layout={[[2], 0.7]} />
				{/each}
			</div>
		{/if}
	{/if}
</div>
