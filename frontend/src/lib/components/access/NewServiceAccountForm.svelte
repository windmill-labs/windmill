<script lang="ts">
	import { Button, Skeleton } from '$lib/components/common'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { FolderService, UserService } from '$lib/gen'
	import { Database, Folder, Info, KeyRound } from 'lucide-svelte'
	import DeployersPicker from './DeployersPicker.svelte'
	import { detectNeeds, type NeedsSource, type RunnableKind } from './detectNeeds'
	import { setPendingServiceAccount, type Grant } from './serviceAccounts.svelte'

	let {
		workspace,
		itemKind,
		itemPath,
		needsSource,
		initialName,
		onUse,
		onBack
	}: {
		workspace: string
		itemKind: RunnableKind
		/** Path of the item being deployed; unset while it has none. */
		itemPath?: string
		/** The item as its editor holds it, read for what it uses. */
		needsSource?: () => NeedsSource | undefined
		/** Typed in the picker's search. */
		initialName?: string
		/** The account is pending: the deploy creates it. */
		onUse: (permissionedAs: string) => void
		onBack: () => void
	} = $props()

	type Row = Grant & { detail: string; checked: boolean }

	const NAME = /^[a-zA-Z0-9_]+$/

	const itemFolder = $derived(
		itemPath?.startsWith('f/') ? itemPath.split('/')[1] || undefined : undefined
	)

	let name = $state('')
	let rows: Row[] = $state([])
	let deployers: string[] = $state([])
	let setDefault = $state(false)
	/** Whether the item's folder already names default users; unknown until loaded. */
	let folderHasDefaults: boolean | undefined = $state(undefined)
	let usernames: string[] = $state([])
	let folderNames: string[] = $state([])
	let detecting = $state(true)
	let detectionErrors: string[] = $state([])

	async function init(): Promise<void> {
		const base = initialName?.trim() || (itemFolder ?? itemPath?.split('/').at(-1) ?? 'service')
		const [users, folders] = await Promise.all([
			UserService.listUsernames({ workspace }).catch(() => [] as string[]),
			FolderService.listFolderNames({ workspace }).catch(() => [] as string[])
		])
		usernames = users
		folderNames = folders
		const wanted = initialName?.trim() ? base : `${base.replace(/[^a-zA-Z0-9_]/g, '_')}_bot`
		let candidate = wanted
		for (let i = 2; users.includes(candidate); i++) candidate = `${wanted}_${i}`
		name = candidate
		if (itemFolder) {
			rows = [
				{
					kind: 'folder',
					path: itemFolder,
					detail: `The ${itemKind}'s folder: everything it uses there`,
					checked: true
				}
			]
			deployers = [`f/${itemFolder}`]
			FolderService.getFolder({ workspace, name: itemFolder }).then(
				(f) => (folderHasDefaults = !!f.default_permissioned_as?.length),
				() => (folderHasDefaults = true)
			)
		}
		if (!itemPath) {
			detecting = false
			return
		}
		try {
			const needs = await detectNeeds(workspace, itemKind, itemPath, needsSource?.())
			detectionErrors = needs.errors
			const outside = needs.items.filter(
				(i) => !itemFolder || !i.path.startsWith(`f/${itemFolder}/`)
			)
			rows = [
				...rows,
				...outside.map((i) => ({
					kind: i.kind,
					path: i.path,
					detail: `${i.kind === 'resource' ? 'Resource' : 'Variable'}, read by ${i.sources[0]}`,
					checked: true
				}))
			]
		} catch (e) {
			detectionErrors = [String(e?.body ?? e)]
		} finally {
			detecting = false
		}
	}
	init()

	const nameError = $derived(
		!name
			? 'Pick a name'
			: !NAME.test(name)
				? 'Letters, digits and _ only'
				: usernames.includes(name)
					? `u/${name} already exists`
					: undefined
	)
	const principal = $derived(`u/${name}`)
	const chosen = $derived(rows.filter((r) => r.checked))
	const reads = $derived(chosen.map((r) => (r.kind === 'folder' ? `f/${r.path}` : r.path)))
	const addableFolders = $derived(
		folderNames
			.filter((f) => !rows.some((r) => r.kind === 'folder' && r.path === f))
			.map((f) => ({ value: f, label: `f/${f}` }))
	)

	function use(): void {
		if (nameError) return
		setPendingServiceAccount(workspace, {
			username: name,
			grants: chosen.map(({ kind, path }) => ({ kind, path })),
			deployers,
			defaultFor: setDefault ? itemFolder : undefined
		})
		onUse(principal)
	}
</script>

<div class="flex flex-col gap-5">
	<p class="text-xs text-secondary">
		A service account can't sign in. The {itemKind} runs with only the access you give it here, and keeps
		it when people leave. Nothing is created until you deploy.
	</p>

	<div class="flex flex-col gap-1">
		<span class="text-xs font-semibold text-emphasis">Name</span>
		<TextInput
			bind:value={name}
			size="sm"
			error={nameError}
			inputProps={{ placeholder: 'billing_bot', 'aria-label': 'Name' }}
		/>
		<span class="text-2xs text-secondary">
			{nameError ?? `Shown as ${principal} in runs and audit logs.`}
		</span>
	</div>

	<div class="flex flex-col gap-2">
		<div class="flex flex-col">
			<span class="text-xs font-semibold text-emphasis">What it can read</span>
			<span class="text-2xs text-secondary"
				>Found in this {itemKind}. Each gets read-only access.</span
			>
		</div>
		{#if detecting}
			<Skeleton layout={[[2], 0.5, [2]]} />
		{:else}
			<div class="flex flex-col rounded-md border divide-y">
				{#each rows as row (row.kind + ':' + row.path)}
					<label class="flex items-start gap-2 px-3 py-2 cursor-pointer">
						<Checkbox
							class="mt-0.5"
							checked={row.checked}
							onChange={() => (row.checked = !row.checked)}
						/>
						{#if row.kind === 'folder'}
							<Folder size={14} class="mt-0.5 shrink-0 text-secondary" />
						{:else if row.kind === 'resource'}
							<Database size={14} class="mt-0.5 shrink-0 text-secondary" />
						{:else}
							<KeyRound size={14} class="mt-0.5 shrink-0 text-secondary" />
						{/if}
						<span class="flex grow flex-col min-w-0">
							<span class="font-mono text-xs text-primary truncate">
								{row.kind === 'folder' ? `f/${row.path}` : row.path}
							</span>
							<span class="text-2xs text-secondary truncate">{row.detail}</span>
						</span>
						<span class="text-2xs text-secondary">Read</span>
					</label>
				{:else}
					<div class="px-3 py-2 text-2xs text-secondary">
						Nothing found. Add the folders it needs below.
					</div>
				{/each}
			</div>
			<div class="w-72">
				<Select
					items={addableFolders}
					bind:value={
						() => undefined,
						(v) =>
							v &&
							(rows = [...rows, { kind: 'folder', path: v, detail: 'Added by you', checked: true }])
					}
					placeholder="Add a folder"
					size="sm"
				/>
			</div>
			<div class="flex items-start gap-1.5 text-2xs text-secondary">
				<Info size={12} class="mt-0.5 shrink-0" />
				<span>
					A path the code builds while it runs can't be found.
					{#if detectionErrors.length}
						Some of it could not be read: {detectionErrors[0]}.
					{/if}
				</span>
			</div>
		{/if}
	</div>

	<div class="flex flex-col gap-2">
		<div class="flex flex-col">
			<span class="text-xs font-semibold text-emphasis">Who can deploy as it</span>
			<span class="text-2xs text-secondary">
				They can make what they deploy run as {name || 'it'}. Anyone else who redeploys this
				{itemKind} makes it run as themselves.
			</span>
		</div>
		<DeployersPicker {principal} bind:deployers {reads} />
	</div>

	{#if itemFolder && folderHasDefaults}
		<span class="text-2xs text-secondary">
			<code>f/{itemFolder}</code> already sets who new items in it run as. Change that in the
			folder's settings on the <a href="/folders">Folders</a> page.
		</span>
	{:else if itemFolder && folderHasDefaults === false}
		<label class="flex items-start gap-2 cursor-pointer">
			<Checkbox
				class="mt-0.5"
				checked={setDefault}
				onChange={() => (setDefault = !setDefault)}
			/>
			<span class="flex flex-col">
				<span class="text-xs text-primary">
					Also run new items in <code>f/{itemFolder}</code> as {name || 'it'}
				</span>
				<span class="text-2xs text-secondary">
					Sets the folder's default user: a new script, flow or app there runs as {name || 'it'}
					when the person creating it can deploy as it.
				</span>
			</span>
		</label>
	{/if}

	<div class="flex items-center justify-between gap-2 border-t pt-3">
		<span class="text-2xs text-secondary">Counts as half a seat.</span>
		<div class="flex gap-2">
			<Button variant="default" unifiedSize="md" onClick={onBack}>Back</Button>
			<Button variant="accent" unifiedSize="md" disabled={!!nameError || detecting} onClick={use}>
				Use {name || 'it'}
			</Button>
		</div>
	</div>
</div>
