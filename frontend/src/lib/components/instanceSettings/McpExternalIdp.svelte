<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { Badge, Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import McpAccessPicker, {
		mcpAccessOf,
		MCP_SESSION,
		RUNNABLES_AS_TOOLS
	} from '../mcp/McpAccessPicker.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { ChevronDown, ChevronRight, Plus, Trash2 } from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'mcp_external_idp'
	const NEW_CLIENT_SCOPES = ['mcp:favorites']

	type ClientRow = {
		id: number
		client_id: string
		workspace_id: string
		scopes: string[]
		/** The workspace `scopes` was picked in: a selection of paths means nothing elsewhere. */
		scopesWorkspace: string
		open: boolean
	}
	type Form = {
		issuer: string
		audience: string
		client_claim: string
		email_claim: string
		clients: ClientRow[]
	}

	let nextId = 0
	// `lastSynced` tells our own writes to `$values[KEY]` apart from external ones (load,
	// save, discard), which re-hydrate the form.
	let form = $state<Form>(toForm(undefined))
	let lastSynced: string | undefined = undefined
	let workspaces = $state<string[]>([])

	WorkspaceService.listWorkspacesAsSuperAdmin()
		.then((ws) => (workspaces = ws.map((w) => w.id)))
		.catch(() => {})

	function toForm(v: any): Form {
		return {
			issuer: v?.issuer ?? '',
			audience: v?.audience ?? '',
			client_claim: v?.client_claim ?? 'azp',
			email_claim: v?.email_claim ?? 'email',
			clients: (v?.clients ?? []).map((c: any) => ({
				id: nextId++,
				client_id: c.client_id ?? '',
				workspace_id: c.workspace_id ?? '',
				scopes: c.scopes ?? [],
				scopesWorkspace: c.workspace_id ?? '',
				open: false
			}))
		}
	}

	// The picker's markers grant nothing, and the server adds `mcp:session` itself.
	function storedScopes(scopes: string[]): string[] {
		return scopes.filter((s) => s !== RUNNABLES_AS_TOOLS && s !== MCP_SESSION)
	}

	function serialize(f: Form): any | undefined {
		if (!f.issuer.trim() && f.clients.length === 0) return undefined
		return {
			issuer: f.issuer.trim(),
			...(f.audience.trim() ? { audience: f.audience.trim() } : {}),
			client_claim: f.client_claim.trim() || 'azp',
			email_claim: f.email_claim.trim() || 'email',
			clients: f.clients.map((c) => ({
				client_id: c.client_id.trim(),
				workspace_id: c.workspace_id,
				scopes: storedScopes(c.scopes)
			}))
		}
	}

	function initialScopes(client: ClientRow): string[] {
		if (client.workspace_id === client.scopesWorkspace) return client.scopes
		return mcpAccessOf(client.scopes) === 'selected' ? NEW_CLIENT_SCOPES : client.scopes
	}

	const ACCESS_LABELS = {
		full: 'Build and run',
		read_run: 'Read and run',
		read: 'Read only'
	}

	function summary(scopes: string[]): string {
		const access = mcpAccessOf(scopes)
		if (access !== 'selected') return ACCESS_LABELS[access]
		const count = (prefix: string) =>
			storedScopes(scopes)
				.filter((s) => s.startsWith(prefix))
				.flatMap((s) => s.slice(prefix.length).split(','))
				.filter((p) => p).length
		const parts = [
			[count('mcp:scripts:'), 'script'],
			[count('mcp:flows:'), 'flow'],
			[count('mcp:hub:'), 'hub script']
		]
			.filter(([n]) => n)
			.map(([n, label]) => `${n} ${label}${n === 1 ? '' : 's'}`)
		if (scopes.includes('mcp:favorites')) parts.unshift('favorites')
		return parts.length ? parts.join(', ') : 'No tools selected'
	}

	$effect(() => {
		const cur = JSON.stringify($values[KEY] ?? null)
		if (cur === lastSynced) return
		lastSynced = cur
		form = toForm($values[KEY])
	})

	$effect(() => {
		const next = serialize(form)
		if (lastSynced === undefined) return
		const nextStr = JSON.stringify(next ?? null)
		if (nextStr === JSON.stringify(serialize(toForm($values[KEY])) ?? null)) return
		lastSynced = nextStr
		$values[KEY] = next
	})
</script>

<div class="flex flex-col gap-4">
	<div class="flex flex-col gap-4 p-4 rounded-md border">
		<label class="flex flex-col gap-1">
			<span class="text-xs text-secondary">Issuer (must equal the token's iss)</span>
			<TextInput
				bind:value={form.issuer}
				inputProps={{ placeholder: 'https://idp.example.com/realms/main', disabled }}
			/>
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-xs text-secondary"
				>Audience (checked against the token's aud; when empty, only tokens typed as access tokens
				are accepted)</span
			>
			<TextInput bind:value={form.audience} inputProps={{ placeholder: 'windmill', disabled }} />
		</label>
		<div class="flex gap-2">
			<label class="flex flex-col gap-1 grow">
				<span class="text-xs text-secondary">Client claim</span>
				<TextInput bind:value={form.client_claim} inputProps={{ placeholder: 'azp', disabled }} />
			</label>
			<label class="flex flex-col gap-1 grow">
				<span class="text-xs text-secondary">User email claim</span>
				<TextInput bind:value={form.email_claim} inputProps={{ placeholder: 'email', disabled }} />
			</label>
		</div>
	</div>

	<div class="flex flex-col gap-2">
		<span class="text-xs font-semibold text-emphasis">MCP clients</span>
		<span class="text-xs text-secondary">
			Each OAuth client of the issuer works in one workspace with the access set here, and never
			beyond what its signed-in user can do.
		</span>
		{#each form.clients as client, i (client.id)}
			<div class="flex flex-col gap-3 p-4 rounded-md border">
				<div class="flex items-center gap-2">
					<button
						type="button"
						class="flex items-center gap-2 grow min-w-0 text-left"
						aria-expanded={client.open}
						onclick={() => (client.open = !client.open)}
					>
						{#if client.open}
							<ChevronDown size={14} class="shrink-0" />
						{:else}
							<ChevronRight size={14} class="shrink-0" />
						{/if}
						<span class="text-xs font-semibold text-emphasis truncate">
							{client.client_id.trim() || 'New client'}
						</span>
						{#if client.workspace_id}
							<Badge small color="gray">{client.workspace_id}</Badge>
						{/if}
						<span class="text-2xs text-secondary truncate">{summary(client.scopes)}</span>
					</button>
					<Button
						variant="subtle"
						unifiedSize="xs"
						startIcon={{ icon: Trash2 }}
						iconOnly
						{disabled}
						aria-label="Remove client"
						onclick={() => form.clients.splice(i, 1)}
					/>
				</div>
				{#if client.open}
					<div class="flex gap-2">
						<label class="flex flex-col gap-1 grow">
							<span class="text-xs text-secondary">Client id</span>
							<TextInput
								bind:value={client.client_id}
								inputProps={{ placeholder: 'my-agent', disabled }}
							/>
						</label>
						<label class="flex flex-col gap-1 w-56">
							<span class="text-xs text-secondary">Workspace</span>
							<Select
								items={workspaces.map((w) => ({ value: w }))}
								bind:value={client.workspace_id}
								{disabled}
							/>
						</label>
					</div>
					{#if client.workspace_id}
						{#key client.workspace_id}
							<McpAccessPicker
								workspaceId={client.workspace_id}
								bind:scopes={client.scopes}
								initialScopes={initialScopes(client)}
							/>
						{/key}
					{/if}
				{/if}
			</div>
		{/each}
		<div class="flex">
			<Button
				variant="default"
				unifiedSize="sm"
				startIcon={{ icon: Plus }}
				{disabled}
				onclick={() =>
					form.clients.push({
						id: nextId++,
						client_id: '',
						workspace_id: '',
						scopes: NEW_CLIENT_SCOPES,
						scopesWorkspace: '',
						open: true
					})}
			>
				Add client
			</Button>
		</div>
	</div>
</div>
