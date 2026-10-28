<script lang="ts">
	import { Button } from '$lib/components/common'
	import Password from '../Password.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import { Plus, Server, Trash2 } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'
	import { enterpriseLicense } from '$lib/stores'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import EEOnly from '../EEOnly.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	type AllowedPrefix = { prefix: string; workspaces: string[] }

	let config = $derived(
		$values['external_secrets'] as
			| { vault?: Record<string, any>; allowed?: AllowedPrefix[] }
			| undefined
	)
	let vault = $derived(config?.vault)
	let allowed = $derived(config?.allowed ?? [])

	let authMethod: 'token' | 'approle' | 'jwt' = $derived(
		vault?.token != null ? 'token' : vault?.approle_role_id != null ? 'approle' : 'jwt'
	)

	let testing = $state(false)

	function update(patch: { vault?: Record<string, any> | undefined; allowed?: AllowedPrefix[] }) {
		const next = { ...(config ?? {}), ...patch }
		$values['external_secrets'] = next.vault || next.allowed?.length ? next : undefined
	}

	function setVaultField(field: string, value: any) {
		update({ vault: { ...vault, [field]: value === '' ? undefined : value } })
	}

	function setAuthMethod(method: string | undefined) {
		if (!vault || !method) return
		const { token: _t, approle_role_id: _a, jwt_role: _j, ...rest } = vault
		update({
			vault: {
				...rest,
				...(method === 'token' ? { token: vault.token ?? '' } : {}),
				...(method === 'approle' ? { approle_role_id: vault.approle_role_id ?? '' } : {}),
				...(method === 'jwt' ? { jwt_role: vault.jwt_role ?? 'windmill-secrets' } : {})
			}
		})
	}

	function setAllowed(i: number, patch: Partial<AllowedPrefix>) {
		update({ allowed: allowed.map((a, j) => (i === j ? { ...a, ...patch } : a)) })
	}

	async function testConnection() {
		if (!vault) return
		testing = true
		try {
			await SettingService.testSecretBackend({
				requestBody: { ...vault, address: vault.address, mount_path: '' }
			})
			sendUserToast('Successfully connected to HashiCorp Vault')
		} catch (e: any) {
			sendUserToast(`Connection failed: ${e.body ?? e.message}`, true)
		} finally {
			testing = false
		}
	}
</script>

<div class="flex flex-col gap-4">
	{#if !$enterpriseLicense}
		<EEOnly>External secret references require Enterprise Edition</EEOnly>
	{/if}
	<span class="text-2xs text-secondary">
		A secret variable can reference an existing secret, as
		<code>$external:vault://&lt;path&gt;#&lt;key&gt;</code>, instead of holding its value. The value
		is read from Vault each time the variable is read. This works whatever the backend type above.
	</span>

	<Toggle
		{disabled}
		checked={!!vault}
		on:change={(e) =>
			update({ vault: e.detail ? { address: '', jwt_role: 'windmill-secrets' } : undefined })}
		size="xs"
		options={{ right: 'HashiCorp Vault connection' }}
	/>

	{#if vault}
		<div class="flex flex-col gap-4 p-4 border rounded-lg">
			<div class="flex flex-col gap-1">
				<label for="ext_vault_address" class="text-xs font-semibold text-emphasis">
					Vault address
				</label>
				<TextInput
					inputProps={{
						id: 'ext_vault_address',
						placeholder: 'https://vault.company.com:8200',
						disabled
					}}
					bind:value={() => vault?.address ?? '', (v) => setVaultField('address', v)}
				/>
			</div>
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-emphasis">Authentication method</span>
				<ToggleButtonGroup selected={authMethod} onSelected={(v) => setAuthMethod(v)}>
					{#snippet children({ item })}
						<ToggleButton
							value="approle"
							label="AppRole"
							tooltip="Log in with an AppRole role_id and secret_id"
							{item}
							{disabled}
						/>
						<ToggleButton
							value="jwt"
							label="JWT auth"
							tooltip="Authenticate using Windmill-signed JWTs"
							{item}
							{disabled}
						/>
						<ToggleButton
							value="token"
							label="Static token"
							tooltip="Use a static Vault token"
							{item}
							{disabled}
						/>
					{/snippet}
				</ToggleButtonGroup>
			</div>
			{#if authMethod === 'approle'}
				<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
					<label for="ext_vault_role_id" class="text-xs font-semibold text-emphasis">Role ID</label>
					<TextInput
						inputProps={{ id: 'ext_vault_role_id', disabled }}
						bind:value={
							() => vault?.approle_role_id ?? '',
							(v) => update({ vault: { ...vault, approle_role_id: v ?? '' } })
						}
					/>
					<span class="text-2xs text-secondary">
						The secret_id is read from the <code>VAULT_APPROLE_SECRET_ID</code> environment variable
						of every server and worker, so it is never stored in Windmill. Leave it unset for a role
						with <code>bind_secret_id=false</code>.
					</span>
					<label for="ext_vault_approle_mount" class="text-xs font-semibold text-emphasis">
						AppRole mount path (optional)
					</label>
					<TextInput
						inputProps={{ id: 'ext_vault_approle_mount', placeholder: 'approle', disabled }}
						bind:value={
							() => vault?.approle_mount_path ?? '', (v) => setVaultField('approle_mount_path', v)
						}
					/>
				</div>
			{:else if authMethod === 'jwt'}
				<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
					<label for="ext_vault_jwt_role" class="text-xs font-semibold text-emphasis">
						JWT auth role
					</label>
					<TextInput
						inputProps={{ id: 'ext_vault_jwt_role', placeholder: 'windmill-secrets', disabled }}
						bind:value={
							() => vault?.jwt_role ?? '', (v) => update({ vault: { ...vault, jwt_role: v ?? '' } })
						}
					/>
					<label for="ext_vault_jwt_mount" class="text-xs font-semibold text-emphasis">
						JWT auth mount path (optional)
					</label>
					<TextInput
						inputProps={{ id: 'ext_vault_jwt_mount', placeholder: 'jwt', disabled }}
						bind:value={
							() => vault?.jwt_mount_path ?? '', (v) => setVaultField('jwt_mount_path', v)
						}
					/>
				</div>
			{:else}
				<div class="flex flex-col gap-1 p-3 bg-surface-secondary rounded-lg">
					<span class="text-xs font-semibold text-emphasis">Vault token</span>
					<span class="text-2xs text-secondary">Recommended only for testing.</span>
					<Password
						bind:password={
							() => vault?.token ?? '', (v) => update({ vault: { ...vault, token: v ?? '' } })
						}
						small
						{disabled}
					/>
				</div>
			{/if}
			<div class="flex flex-col gap-1">
				<label for="ext_vault_namespace" class="text-xs font-semibold text-emphasis">
					Namespace (optional)
				</label>
				<TextInput
					inputProps={{ id: 'ext_vault_namespace', placeholder: 'admin/my-namespace', disabled }}
					bind:value={() => vault?.namespace ?? '', (v) => setVaultField('namespace', v)}
				/>
			</div>
			<Toggle
				{disabled}
				checked={!!vault?.skip_ssl_verify}
				on:change={(e) => setVaultField('skip_ssl_verify', e.detail || undefined)}
				size="xs"
				options={{ right: 'Skip TLS certificate verification' }}
			/>
			<Button
				unifiedSize="md"
				variant="default"
				onclick={testConnection}
				disabled={disabled || !vault?.address || testing}
				loading={testing}
				startIcon={{ icon: Server }}>Test connection</Button
			>
		</div>
	{/if}

	<div class="flex flex-col gap-2">
		<span class="text-xs font-semibold text-emphasis">Allowed prefixes</span>
		<span class="text-2xs text-secondary">
			A reference resolves only under one of these prefixes, matched on whole path segments, for the
			listed workspaces (all workspaces when empty). Anyone who can create a variable in a workspace
			can read every secret under its prefixes, within what Windmill's Vault identity can read.
		</span>
		{#each allowed as a, i (i)}
			<div class="flex flex-row gap-2 items-center">
				<TextInput
					class="flex-1"
					inputProps={{ placeholder: 'vault://secret/team', disabled }}
					bind:value={() => a.prefix, (v) => setAllowed(i, { prefix: v ?? '' })}
				/>
				<TextInput
					class="flex-1"
					inputProps={{
						placeholder: 'All workspaces (or ws1, ws2)',
						disabled,
						// Parsed on change, not per keystroke, so a trailing comma survives typing.
						onchange: (e) =>
							setAllowed(i, {
								workspaces: e.currentTarget.value
									.split(',')
									.map((w) => w.trim())
									.filter(Boolean)
							})
					}}
					value={a.workspaces.join(', ')}
				/>
				<Button
					unifiedSize="md"
					variant="subtle"
					iconOnly
					startIcon={{ icon: Trash2 }}
					{disabled}
					aria-label="Remove prefix"
					onclick={() => update({ allowed: allowed.filter((_, j) => j !== i) })}
				/>
			</div>
		{/each}
		<Button
			unifiedSize="sm"
			variant="default"
			startIcon={{ icon: Plus }}
			{disabled}
			wrapperClasses="self-start"
			onclick={() => update({ allowed: [...allowed, { prefix: 'vault://', workspaces: [] }] })}
			>Add prefix</Button
		>
	</div>
</div>
