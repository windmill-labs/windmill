<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Select from '$lib/components/select/Select.svelte'
	import {
		FolderService,
		GranularAclService,
		GroupService,
		UserService,
		type PrincipalAccess
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Folder, User, Users, X } from 'lucide-svelte'

	let {
		principal,
		access
	}: {
		/** `u/<service account>` */
		principal: string
		access: PrincipalAccess
	} = $props()

	let saved: string[] = $state([])
	let deployers: string[] = $state([])
	let options: { value: string; label: string }[] = $state([])
	let saving = $state(false)
	let loaded = $state(false)

	async function load(): Promise<void> {
		const workspace = $workspaceStore!
		const [list, users, groups, folders] = await Promise.all([
			GranularAclService.getServiceAccountDeployers({ workspace, principal }),
			UserService.listUsernames({ workspace }),
			GroupService.listGroupNames({ workspace }),
			FolderService.listFolderNames({ workspace })
		])
		saved = list.deployers
		deployers = [...list.deployers]
		options = [
			...users.filter((u) => `u/${u}` !== principal).map((u) => `u/${u}`),
			...groups.map((g) => `g/${g}`),
			...folders.map((f) => `f/${f}`)
		].map((value) => ({
			value,
			label: value.startsWith('f/') ? `${value} (whoever can write in it)` : value
		}))
		loaded = true
	}
	load().catch((e) => sendUserToast(`Could not load deployers: ${e?.body ?? e}`, true))

	const available = $derived(options.filter((o) => !deployers.includes(o.value)))
	const dirty = $derived(
		deployers.length !== saved.length || deployers.some((d) => !saved.includes(d))
	)

	// Folders on the list are reachable by their writers already: what the list hands over beyond
	// that is everything else the account reads.
	const listedFolders = $derived(deployers.filter((d) => d.startsWith('f/')).map((d) => `${d}/`))
	const beyond = $derived(
		access.items.filter((i) => !listedFolders.some((f) => i.path.startsWith(f))).map((i) => i.path)
	)

	async function save(): Promise<void> {
		saving = true
		try {
			await GranularAclService.setServiceAccountDeployers({
				workspace: $workspaceStore!,
				requestBody: { principal, deployers }
			})
			saved = [...deployers]
			sendUserToast(`Saved who can deploy as ${principal}`)
		} catch (e) {
			sendUserToast(`Could not save: ${e?.body ?? e}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div class="flex flex-col gap-2">
	<p class="text-2xs text-secondary">
		These people can deploy scripts, flows, apps, schedules and triggers that run as {principal}.
		Workspace admins and members of wm_deployers always can. Anyone else who deploys an item set to
		run as {principal} makes it run as themselves.
	</p>
	{#if loaded}
		<div class="flex flex-wrap items-center gap-1">
			{#each deployers as entry (entry)}
				<span
					class="inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-2xs text-primary"
				>
					{#if entry.startsWith('f/')}
						<Folder size={12} />
					{:else if entry.startsWith('g/')}
						<Users size={12} />
					{:else}
						<User size={12} />
					{/if}
					{entry}
					<Button
						variant="subtle"
						unifiedSize="2xs"
						iconOnly
						startIcon={{ icon: X }}
						title={`Remove ${entry}`}
						onClick={() => (deployers = deployers.filter((d) => d !== entry))}
					/>
				</span>
			{:else}
				<span class="text-2xs text-secondary">Only admins and wm_deployers.</span>
			{/each}
		</div>
		<div class="flex items-center gap-2">
			<div class="w-72">
				<Select
					items={available}
					bind:value={() => undefined, (v) => v && (deployers = [...deployers, v])}
					placeholder="Add a user, group or folder"
					size="sm"
				/>
			</div>
			<Button variant="accent" unifiedSize="sm" disabled={!dirty} loading={saving} onClick={save}>
				Save
			</Button>
		</div>
		{#if deployers.length && beyond.length}
			<Alert type="warning" size="xs" title={`They can read what ${principal} reads`}>
				Through {principal}, people on this list can read {beyond.length}
				{beyond.length === 1 ? 'item' : 'items'} outside the folders it names: {beyond
					.slice(0, 5)
					.join(', ')}{beyond.length > 5 ? ', …' : ''}.
			</Alert>
		{/if}
	{/if}
</div>
