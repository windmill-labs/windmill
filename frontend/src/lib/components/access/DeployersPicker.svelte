<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import { FolderService, GroupService, UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Folder, User, Users, X } from 'lucide-svelte'

	let {
		principal,
		deployers = $bindable(),
		reads
	}: {
		/** `u/<service account>` */
		principal: string
		deployers: string[]
		/** Paths of the items and folders (`f/<name>`) the account reads. */
		reads: string[]
	} = $props()

	let options: { value: string; label: string }[] = $state([])

	async function loadOptions(): Promise<void> {
		const workspace = $workspaceStore!
		const [users, groups, folders] = await Promise.all([
			UserService.listUsernames({ workspace }),
			GroupService.listGroupNames({ workspace }),
			FolderService.listFolderNames({ workspace })
		])
		options = [
			...users.filter((u) => `u/${u}` !== principal).map((u) => `u/${u}`),
			...groups.map((g) => `g/${g}`),
			...folders.map((f) => `f/${f}`)
		].map((value) => ({
			value,
			label: value.startsWith('f/') ? `${value} (whoever can write in it)` : value
		}))
	}
	loadOptions().catch(() => (options = []))

	const available = $derived(options.filter((o) => !deployers.includes(o.value)))

	// Folders on the list are reachable by their writers already: what the list hands over beyond
	// that is everything else the account reads.
	const listedFolders = $derived(deployers.filter((d) => d.startsWith('f/')))
	const beyond = $derived(
		reads.filter((p) => !listedFolders.some((f) => p === f || p.startsWith(`${f}/`)))
	)
</script>

<div class="flex flex-col gap-2">
	<div class="flex flex-wrap items-center gap-1">
		{#each deployers as entry (entry)}
			<span
				class="inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-2xs text-primary"
			>
				{#if entry.startsWith('f/')}
					<Folder size={12} />
					Writers of {entry}
				{:else if entry.startsWith('g/')}
					<Users size={12} />
					{entry}
				{:else}
					<User size={12} />
					{entry}
				{/if}
				<Button
					variant="subtle"
					unifiedSize="2xs"
					iconOnly
					startIcon={{ icon: X }}
					title={`Remove ${entry}`}
					onClick={() => (deployers = deployers.filter((d) => d !== entry))}
				/>
			</span>
		{/each}
		<span class="text-2xs text-secondary">
			{deployers.length ? '+ admins and wm_deployers' : 'Only admins and wm_deployers.'}
		</span>
	</div>
	<div class="w-72">
		<Select
			items={available}
			bind:value={() => undefined, (v) => v && (deployers = [...deployers, v])}
			placeholder="Add a user, group or folder"
			size="sm"
		/>
	</div>
	{#if deployers.length && beyond.length}
		<Alert type="warning" size="xs" title={`They can read what ${principal} reads`}>
			Through {principal}, people on this list can read {beyond.length}
			{beyond.length === 1 ? 'item' : 'items'} outside the folders it names: {beyond
				.slice(0, 5)
				.join(', ')}{beyond.length > 5 ? ', …' : ''}.
		</Alert>
	{/if}
</div>
