<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import McpScopeSelector from '../mcp/McpScopeSelector.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { Plus, Trash2 } from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'mcp_external_idp'

	type ClientRow = { id: number; client_id: string; workspace_id: string; scope: string }
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
				scope: (c.scopes ?? []).join(' ')
			}))
		}
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
				scopes: c.scope.split(/\s+/).filter((s) => s)
			}))
		}
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
			>Audience (checked against the token's aud; when empty, only tokens typed as access tokens are
			accepted)</span
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
	<div class="flex flex-col gap-2">
		<span class="text-xs font-semibold text-emphasis">MCP clients</span>
		<span class="text-xs text-secondary">
			Each OAuth client of the issuer is bound to one workspace and offered only the tools selected
			here, and only those its user can also see and run.
		</span>
		{#each form.clients as client, i (client.id)}
			<div class="flex flex-col gap-2 border-l-2 border-surface-selected pl-3">
				<div class="flex items-end gap-2">
					<label class="flex flex-col gap-1 grow">
						<span class="text-2xs text-secondary">Client id</span>
						<TextInput
							bind:value={client.client_id}
							inputProps={{ placeholder: 'my-agent', disabled }}
						/>
					</label>
					<label class="flex flex-col gap-1 w-56">
						<span class="text-2xs text-secondary">Workspace</span>
						<Select
							items={workspaces.map((w) => ({ value: w }))}
							bind:value={client.workspace_id}
							{disabled}
						/>
					</label>
					<Button
						variant="subtle"
						unifiedSize="md"
						startIcon={{ icon: Trash2 }}
						iconOnly
						{disabled}
						aria-label="Remove client"
						onclick={() => form.clients.splice(i, 1)}
					/>
				</div>
				{#if client.workspace_id}
					{#key client.workspace_id}
						<McpScopeSelector
							workspaceId={client.workspace_id}
							bind:scope={client.scope}
							initialScope={client.scope}
							runnablesOnly
						/>
					{/key}
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
					form.clients.push({ id: nextId++, client_id: '', workspace_id: '', scope: '' })}
			>
				Add client
			</Button>
		</div>
	</div>
</div>
