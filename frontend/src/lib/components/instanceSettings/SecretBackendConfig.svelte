<script lang="ts">
	import { Button } from '$lib/components/common'
	import Password from '../Password.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import { Database, Lock, Server, ArrowLeft, ArrowRight, Cloud } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'
	import { enterpriseLicense } from '$lib/stores'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import EEOnly from '../EEOnly.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	$effect(() => {
		if (!$values['secret_backend']) {
			$values['secret_backend'] = { type: 'Database' }
		}
	})

	let selectedType: 'Database' | 'HashiCorpVault' | 'AzureKeyVault' | 'AwsSecretsManager' = $derived(
		$values['secret_backend']?.type ?? 'Database'
	)

	let authMethod: 'token' | 'jwt' = $derived.by(() => {
		const config = $values['secret_backend']
		if (!config || config.type !== 'HashiCorpVault') return 'jwt'
		return config.jwt_role === null ? 'token' : 'jwt'
	})

	let testingConnection = $state(false)
	let migratingToVault = $state(false)
	let migratingToDatabase = $state(false)
	let migrateToVaultModalOpen = $state(false)
	let migrateToDatabaseModalOpen = $state(false)

	let testingAzureKvConnection = $state(false)
	let migratingToAzureKv = $state(false)
	let migratingFromAzureKv = $state(false)
	let migrateToAzureKvModalOpen = $state(false)
	let migrateFromAzureKvModalOpen = $state(false)

	let testingAwsSmConnection = $state(false)
	let migratingToAwsSm = $state(false)
	let migratingFromAwsSm = $state(false)
	let migrateToAwsSmModalOpen = $state(false)
	let migrateFromAwsSmModalOpen = $state(false)

	let vaultDisabled = $derived(!$enterpriseLicense)

	function setBackendType(type: string | undefined) {
		if (!type) return
		if ((type === 'HashiCorpVault' || type === 'AzureKeyVault' || type === 'AwsSecretsManager') && vaultDisabled) return
		if (type === 'Database') {
			$values['secret_backend'] = { type: 'Database' }
		} else if (type === 'HashiCorpVault') {
			$values['secret_backend'] = {
				type: 'HashiCorpVault',
				address: $values['secret_backend']?.address ?? '',
				mount_path: $values['secret_backend']?.mount_path ?? 'windmill',
				jwt_role: $values['secret_backend']?.jwt_role ?? 'windmill-secrets',
				namespace: $values['secret_backend']?.namespace ?? null,
				token: $values['secret_backend']?.token ?? null,
				skip_ssl_verify: $values['secret_backend']?.skip_ssl_verify ?? false
			}
		} else if (type === 'AzureKeyVault') {
			$values['secret_backend'] = {
				type: 'AzureKeyVault',
				vault_url: $values['secret_backend']?.vault_url ?? '',
				tenant_id: $values['secret_backend']?.tenant_id ?? '',
				client_id: $values['secret_backend']?.client_id ?? '',
				client_secret: $values['secret_backend']?.client_secret ?? null,
				token: $values['secret_backend']?.token ?? null
			}
		} else if (type === 'AwsSecretsManager') {
			$values['secret_backend'] = {
				type: 'AwsSecretsManager',
				region: $values['secret_backend']?.region ?? 'us-east-1',
				access_key_id: $values['secret_backend']?.access_key_id ?? null,
				secret_access_key: $values['secret_backend']?.secret_access_key ?? null,
				endpoint_url: $values['secret_backend']?.endpoint_url ?? null,
				prefix: $values['secret_backend']?.prefix ?? 'windmill/'
			}
		}
	}

	function setAuthMethod(method: string | undefined) {
		if (!method || !$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') return
		if (method === 'token') {
			$values['secret_backend'] = { ...$values['secret_backend'], jwt_role: null, token: $values['secret_backend'].token ?? '' }
		} else if (method === 'jwt') {
			$values['secret_backend'] = { ...$values['secret_backend'], token: null, jwt_role: $values['secret_backend'].jwt_role ?? 'windmill-secrets' }
		}
	}

	function getVaultSettings() {
		return {
			address: $values['secret_backend'].address,
			mount_path: $values['secret_backend'].mount_path,
			jwt_role: $values['secret_backend'].jwt_role,
			namespace: $values['secret_backend'].namespace || undefined,
			token: $values['secret_backend'].token || undefined,
			skip_ssl_verify: $values['secret_backend'].skip_ssl_verify || undefined
		}
	}

	async function testVaultConnection() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') return
		testingConnection = true
		try {
			await SettingService.testSecretBackend({ requestBody: getVaultSettings() })
			sendUserToast('Successfully connected to HashiCorp Vault')
		} catch (error: any) {
			sendUserToast('Failed to connect to Vault: ' + error.message, true)
		} finally { testingConnection = false }
	}

	async function migrateSecretsToVault() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') return
		migratingToVault = true
		try {
			const report = await SettingService.migrateSecretsToVault({ requestBody: getVaultSettings() })
			if (report.failed_count > 0) sendUserToast(`Migration: ${report.migrated_count}/${report.total_secrets} migrated, ${report.failed_count} failed`, true)
			else sendUserToast(`Migrated ${report.migrated_count}/${report.total_secrets} secrets to Vault`)
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { migratingToVault = false; migrateToVaultModalOpen = false }
	}

	async function migrateSecretsToDatabase() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') return
		migratingToDatabase = true
		try {
			const report = await SettingService.migrateSecretsToDatabase({ requestBody: getVaultSettings() })
			if (report.failed_count > 0) sendUserToast(`Migration: ${report.migrated_count}/${report.total_secrets} migrated, ${report.failed_count} failed`, true)
			else sendUserToast(`Migrated ${report.migrated_count}/${report.total_secrets} secrets to database`)
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { migratingToDatabase = false; migrateToDatabaseModalOpen = false }
	}

	function isVaultConfigValid(): boolean {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') return false
		const hasAddress = $values['secret_backend'].address?.trim() !== ''
		const hasMountPath = $values['secret_backend'].mount_path?.trim() !== ''
		const hasToken = $values['secret_backend'].token?.trim()
		const hasJwtRole = $values['secret_backend'].jwt_role?.trim()
		return hasAddress && hasMountPath && (hasToken || hasJwtRole)
	}

	function getAzureKvSettings() {
		return {
			vault_url: $values['secret_backend'].vault_url,
			tenant_id: $values['secret_backend'].tenant_id,
			client_id: $values['secret_backend'].client_id,
			client_secret: $values['secret_backend'].client_secret || undefined,
			token: $values['secret_backend'].token || undefined
		}
	}

	async function testAzureKvConnection() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AzureKeyVault') return
		testingAzureKvConnection = true
		try {
			await SettingService.testAzureKvBackend({ requestBody: getAzureKvSettings() })
			sendUserToast('Successfully connected to Azure Key Vault')
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { testingAzureKvConnection = false }
	}

	async function migrateSecretsToAzureKv() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AzureKeyVault') return
		migratingToAzureKv = true
		try {
			const report = await SettingService.migrateSecretsToAzureKv({ requestBody: getAzureKvSettings() })
			if (report.failed_count > 0) sendUserToast(`Migration: ${report.migrated_count}/${report.total_secrets} migrated, ${report.failed_count} failed`, true)
			else sendUserToast(`Migrated ${report.migrated_count}/${report.total_secrets} secrets to Azure Key Vault`)
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { migratingToAzureKv = false; migrateToAzureKvModalOpen = false }
	}

	async function migrateSecretsFromAzureKv() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AzureKeyVault') return
		migratingFromAzureKv = true
		try {
			const report = await SettingService.migrateSecretsFromAzureKv({ requestBody: getAzureKvSettings() })
			if (report.failed_count > 0) sendUserToast(`Migration: ${report.migrated_count}/${report.total_secrets} migrated, ${report.failed_count} failed`, true)
			else sendUserToast(`Migrated ${report.migrated_count}/${report.total_secrets} secrets to database`)
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { migratingFromAzureKv = false; migrateFromAzureKvModalOpen = false }
	}

	function isAzureKvConfigValid(): boolean {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AzureKeyVault') return false
		return (
			$values['secret_backend'].vault_url?.trim() !== '' &&
			$values['secret_backend'].tenant_id?.trim() !== '' &&
			$values['secret_backend'].client_id?.trim() !== '' &&
			(!!$values['secret_backend'].client_secret?.trim() || !!$values['secret_backend'].token?.trim())
		)
	}

	function getAwsSmSettings() {
		return {
			region: $values['secret_backend'].region,
			access_key_id: $values['secret_backend'].access_key_id || undefined,
			secret_access_key: $values['secret_backend'].secret_access_key || undefined,
			endpoint_url: $values['secret_backend'].endpoint_url || undefined,
			prefix: $values['secret_backend'].prefix || undefined
		}
	}

	async function testAwsSmConnection() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AwsSecretsManager') return
		testingAwsSmConnection = true
		try {
			await SettingService.testAwsSmBackend({ requestBody: getAwsSmSettings() })
			sendUserToast('Successfully connected to AWS Secrets Manager')
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { testingAwsSmConnection = false }
	}

	async function migrateSecretsToAwsSm() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AwsSecretsManager') return
		migratingToAwsSm = true
		try {
			const report = await SettingService.migrateSecretsToAwsSm({ requestBody: getAwsSmSettings() })
			if (report.failed_count > 0) sendUserToast(`Migration: ${report.migrated_count}/${report.total_secrets} migrated, ${report.failed_count} failed`, true)
			else sendUserToast(`Migrated ${report.migrated_count}/${report.total_secrets} secrets to AWS Secrets Manager`)
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { migratingToAwsSm = false; migrateToAwsSmModalOpen = false }
	}

	async function migrateSecretsFromAwsSm() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AwsSecretsManager') return
		migratingFromAwsSm = true
		try {
			const report = await SettingService.migrateSecretsFromAwsSm({ requestBody: getAwsSmSettings() })
			if (report.failed_count > 0) sendUserToast(`Migration: ${report.migrated_count}/${report.total_secrets} migrated, ${report.failed_count} failed`, true)
			else sendUserToast(`Migrated ${report.migrated_count}/${report.total_secrets} secrets to database`)
		} catch (error: any) { sendUserToast('Failed: ' + error.message, true) }
		finally { migratingFromAwsSm = false; migrateFromAwsSmModalOpen = false }
	}

	function isAwsSmConfigValid(): boolean {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'AwsSecretsManager') return false
		return $values['secret_backend'].region?.trim() !== ''
	}

	let baseUrl = $derived($values['base_url'] ?? 'https://your-windmill-instance.com')
</script>

<div class="space-y-6">
	<div class="flex flex-col gap-2 mt-1">
		<ToggleButtonGroup selected={selectedType} onSelected={(v) => setBackendType(v)}>
			{#snippet children({ item: toggleButton })}
				<ToggleButton value="Database" label="Database" tooltip="Store secrets encrypted in the database (default)" item={toggleButton} />
				<ToggleButton value="HashiCorpVault" label="HashiCorp Vault (Beta)" tooltip={vaultDisabled ? 'Requires Enterprise Edition' : 'Store secrets in HashiCorp Vault'} item={toggleButton} disabled={vaultDisabled} />
				<ToggleButton value="AzureKeyVault" label="Azure Key Vault" tooltip={vaultDisabled ? 'Requires Enterprise Edition' : 'Store secrets in Azure Key Vault'} item={toggleButton} disabled={vaultDisabled} />
				<ToggleButton value="AwsSecretsManager" label="AWS Secrets Manager (Beta)" tooltip={vaultDisabled ? 'Requires Enterprise Edition' : 'Store secrets in AWS Secrets Manager'} item={toggleButton} disabled={vaultDisabled} />
			{/snippet}
		</ToggleButtonGroup>
		{#if vaultDisabled}
			<div class="flex items-center gap-1">
				<EEOnly>External secret store integrations require Enterprise Edition</EEOnly>
			</div>
		{/if}
	</div>

	{#if selectedType === 'Database'}
		<div class="flex items-center gap-2 p-4 bg-surface-secondary rounded-lg">
			<Database class="text-primary" size={20} />
			<div>
				<p class="text-sm font-medium text-emphasis">Database Storage (Default)</p>
				<p class="text-xs text-secondary">Secrets are encrypted using workspace-specific keys and stored in the PostgreSQL database.</p>
			</div>
		</div>
	{:else if selectedType === 'HashiCorpVault'}
		<div class="space-y-4 p-4 border rounded-lg">
			<div class="flex items-center gap-2 mb-4">
				<Lock class="text-primary" size={20} />
				<div>
					<p class="text-sm font-medium text-emphasis">HashiCorp Vault Configuration <span class="ml-2 px-1.5 py-0.5 text-2xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200 rounded">Beta</span></p>
					<p class="text-xs text-secondary">Store secrets in an external HashiCorp Vault instance.</p>
				</div>
			</div>
			<div class="grid grid-cols-1 gap-4">
				<div class="flex flex-col gap-1">
					<label for="vault_address" class="block text-xs font-semibold text-emphasis">Vault Address</label>
					<TextInput inputProps={{ type: 'text', id: 'vault_address', placeholder: 'https://vault.company.com:8200', disabled }} bind:value={$values['secret_backend'].address} />
				</div>
				<div class="flex flex-col gap-1">
					<label for="vault_mount_path" class="block text-xs font-semibold text-emphasis">KV Mount Path</label>
					<span class="text-2xs text-secondary">The KV v2 secrets engine mount path in Vault</span>
					<TextInput inputProps={{ type: 'text', id: 'vault_mount_path', placeholder: 'windmill', disabled }} bind:value={$values['secret_backend'].mount_path} />
				</div>
				<div class="flex flex-col gap-2">
					<span class="block text-xs font-semibold text-emphasis">Authentication Method</span>
					<ToggleButtonGroup selected={authMethod} onSelected={(v) => setAuthMethod(v)}>
						{#snippet children({ item: toggleButton })}
							<ToggleButton value="jwt" label="JWT Auth" tooltip="Authenticate using Windmill-signed JWTs" item={toggleButton} {disabled} />
							<ToggleButton value="token" label="Static Token" tooltip="Use a static Vault token" item={toggleButton} {disabled} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
				{#if authMethod === 'token'}
					<div class="flex flex-col gap-1 p-3 bg-surface-secondary rounded-lg">
						<label for="vault_token" class="block text-xs font-semibold text-emphasis">Vault Token</label>
						<span class="text-2xs text-secondary">Static token. Recommended only for testing/development.</span>
						<Password bind:password={$values['secret_backend'].token} small {disabled} />
					</div>
				{:else}
					<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
						<label for="vault_jwt_role" class="block text-xs font-semibold text-emphasis">JWT Auth Role</label>
						<span class="text-2xs text-secondary">The JWT authentication role configured in Vault.</span>
						<TextInput inputProps={{ type: 'text', id: 'vault_jwt_role', placeholder: 'windmill-secrets', disabled }} bind:value={$values['secret_backend'].jwt_role} />
						<details class="mt-2">
							<summary class="text-xs font-medium text-secondary cursor-pointer hover:text-primary">Vault JWT Setup Instructions</summary>
							<div class="mt-2 p-2 bg-surface rounded text-2xs text-secondary space-y-2">
								<p>Configure Vault to accept JWTs from Windmill:</p>
								<div class="bg-gray-100 dark:bg-gray-800 p-2 rounded font-mono text-2xs overflow-x-auto">
									<pre># Enable JWT auth method
vault auth enable jwt

# Configure JWT auth with Windmill's JWKS endpoint
vault write auth/jwt/config \
  jwks_url="{baseUrl}/.well-known/jwks.json" \
  bound_issuer="{baseUrl}"

# Create a policy for Windmill secrets
vault policy write windmill-secrets - &lt;&lt;EOF
path "windmill/data/*" &#123;
  capabilities = ["create", "read", "update", "delete"]
&#125;
path "windmill/metadata/*" &#123;
  capabilities = ["list", "delete"]
&#125;
EOF

# Create the JWT role
vault write auth/jwt/role/windmill-secrets \
  role_type="jwt" \
  bound_audiences="{baseUrl}" \
  user_claim="email" \
  policies="windmill-secrets" \
  ttl="1h"</pre>
								</div>
							</div>
						</details>
					</div>
				{/if}
				<div class="flex flex-col gap-1">
					<label for="vault_namespace" class="block text-xs font-semibold text-emphasis">Namespace (optional)</label>
					<span class="text-2xs text-secondary">Vault Enterprise namespace</span>
					<TextInput inputProps={{ type: 'text', id: 'vault_namespace', placeholder: 'admin/my-namespace', disabled }} bind:value={$values['secret_backend'].namespace} />
				</div>
				<div class="flex flex-col gap-1">
					<Toggle id="vault_skip_ssl_verify" {disabled} bind:checked={$values['secret_backend'].skip_ssl_verify} size="xs" options={{ right: 'Skip TLS certificate verification' }} />
					<span class="text-2xs text-secondary">Disables TLS verification when connecting to Vault. Only enable for self-signed certificates in development.</span>
				</div>
			</div>
			<div class="flex flex-col gap-4 pt-4 border-t">
				<Button unifiedSize="md" variant="accent" onclick={testVaultConnection} disabled={disabled || !isVaultConfigValid() || testingConnection} loading={testingConnection} startIcon={{ icon: Server }}>Test Connection</Button>
				<div class="flex flex-col gap-4 pt-4 border-t">
					<span class="block text-xs font-semibold text-emphasis">Secret Migration</span>
					<span class="text-2xs text-secondary">Original values are NOT deleted to allow for rollback.</span>
					<div class="flex gap-4">
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2"><Database size={16} /><ArrowRight size={16} /><Lock size={16} /></div>
							<p class="text-xs font-medium mb-2">Database → Vault</p>
							<Button unifiedSize="sm" variant="default" onclick={() => (migrateToVaultModalOpen = true)} disabled={disabled || !isVaultConfigValid() || migratingToVault} startIcon={{ icon: ArrowRight }}>Migrate to Vault</Button>
						</div>
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2"><Lock size={16} /><ArrowLeft size={16} /><Database size={16} /></div>
							<p class="text-xs font-medium mb-2">Vault → Database</p>
							<Button unifiedSize="sm" variant="default" onclick={() => (migrateToDatabaseModalOpen = true)} disabled={disabled || !isVaultConfigValid() || migratingToDatabase} startIcon={{ icon: ArrowLeft }}>Migrate to Database</Button>
						</div>
					</div>
				</div>
			</div>
		</div>
	{:else if selectedType === 'AzureKeyVault'}
		<div class="space-y-4 p-4 border rounded-lg">
			<div class="flex items-center gap-2 mb-4">
				<Cloud class="text-primary" size={20} />
				<div>
					<p class="text-sm font-medium text-emphasis">Azure Key Vault Configuration</p>
					<p class="text-xs text-secondary">Store secrets in an Azure Key Vault instance.</p>
				</div>
			</div>
			<div class="grid grid-cols-1 gap-4">
				<div class="flex flex-col gap-1">
					<label for="azure_vault_url" class="block text-xs font-semibold text-emphasis">Vault URL</label>
					<TextInput inputProps={{ type: 'text', id: 'azure_vault_url', placeholder: 'https://my-vault.vault.azure.net', disabled }} bind:value={$values['secret_backend'].vault_url} />
				</div>
				<div class="flex flex-col gap-1">
					<label for="azure_tenant_id" class="block text-xs font-semibold text-emphasis">Tenant ID</label>
					<TextInput inputProps={{ type: 'text', id: 'azure_tenant_id', placeholder: '00000000-0000-0000-0000-000000000000', disabled }} bind:value={$values['secret_backend'].tenant_id} />
				</div>
				<div class="flex flex-col gap-1">
					<label for="azure_client_id" class="block text-xs font-semibold text-emphasis">Client ID</label>
					<TextInput inputProps={{ type: 'text', id: 'azure_client_id', placeholder: '00000000-0000-0000-0000-000000000000', disabled }} bind:value={$values['secret_backend'].client_id} />
				</div>
				<div class="flex flex-col gap-1 p-3 bg-surface-secondary rounded-lg">
					<label for="azure_client_secret" class="block text-xs font-semibold text-emphasis">Client Secret</label>
					<Password bind:password={$values['secret_backend'].client_secret} small {disabled} />
				</div>
				<div class="flex flex-col gap-1 p-3 bg-surface-secondary rounded-lg">
					<label for="azure_token" class="block text-xs font-semibold text-emphasis">Token (optional)</label>
					<span class="text-2xs text-secondary">Static Bearer token for testing. If provided, OAuth2 is skipped.</span>
					<Password bind:password={$values['secret_backend'].token} small {disabled} />
				</div>
			</div>
			<div class="flex flex-col gap-4 pt-4 border-t">
				<Button unifiedSize="md" variant="accent" onclick={testAzureKvConnection} disabled={disabled || !isAzureKvConfigValid() || testingAzureKvConnection} loading={testingAzureKvConnection} startIcon={{ icon: Server }}>Test Connection</Button>
				<div class="flex flex-col gap-4 pt-4 border-t">
					<span class="block text-xs font-semibold text-emphasis">Secret Migration</span>
					<span class="text-2xs text-secondary">Original values are NOT deleted to allow for rollback.</span>
					<div class="flex gap-4">
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2"><Database size={16} /><ArrowRight size={16} /><Cloud size={16} /></div>
							<p class="text-xs font-medium mb-2">Database → Azure Key Vault</p>
							<Button unifiedSize="sm" variant="default" onclick={() => (migrateToAzureKvModalOpen = true)} disabled={disabled || !isAzureKvConfigValid() || migratingToAzureKv} startIcon={{ icon: ArrowRight }}>Migrate to Azure KV</Button>
						</div>
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2"><Cloud size={16} /><ArrowLeft size={16} /><Database size={16} /></div>
							<p class="text-xs font-medium mb-2">Azure Key Vault → Database</p>
							<Button unifiedSize="sm" variant="default" onclick={() => (migrateFromAzureKvModalOpen = true)} disabled={disabled || !isAzureKvConfigValid() || migratingFromAzureKv} startIcon={{ icon: ArrowLeft }}>Migrate to Database</Button>
						</div>
					</div>
				</div>
			</div>
		</div>
	{:else if selectedType === 'AwsSecretsManager'}
		<div class="space-y-4 p-4 border rounded-lg">
			<div class="flex items-center gap-2 mb-4">
				<Cloud class="text-primary" size={20} />
				<div>
					<p class="text-sm font-medium text-emphasis">AWS Secrets Manager Configuration <span class="ml-2 px-1.5 py-0.5 text-2xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200 rounded">Beta</span></p>
					<p class="text-xs text-secondary">Store secrets in AWS Secrets Manager.</p>
				</div>
			</div>
			<div class="grid grid-cols-1 gap-4">
				<div class="flex flex-col gap-1">
					<label for="aws_sm_region" class="block text-xs font-semibold text-emphasis">Region</label>
					<TextInput inputProps={{ type: 'text', id: 'aws_sm_region', placeholder: 'us-east-1', disabled }} bind:value={$values['secret_backend'].region} />
				</div>
				<div class="flex flex-col gap-1">
					<label for="aws_sm_access_key_id" class="block text-xs font-semibold text-emphasis">Access Key ID (optional)</label>
					<span class="text-2xs text-secondary">If not provided, the default AWS credential chain is used (env vars, instance profile, EKS pod identity)</span>
					<TextInput inputProps={{ type: 'text', id: 'aws_sm_access_key_id', placeholder: 'AKIA...', disabled }} bind:value={$values['secret_backend'].access_key_id} />
				</div>
				<div class="flex flex-col gap-1 p-3 bg-surface-secondary rounded-lg">
					<label for="aws_sm_secret_access_key" class="block text-xs font-semibold text-emphasis">Secret Access Key (optional)</label>
					<Password bind:password={$values['secret_backend'].secret_access_key} small {disabled} />
				</div>
				<div class="flex flex-col gap-1">
					<label for="aws_sm_prefix" class="block text-xs font-semibold text-emphasis">Secret Name Prefix (optional)</label>
					<span class="text-2xs text-secondary">Prefix for secret names in AWS Secrets Manager (default: windmill/)</span>
					<TextInput inputProps={{ type: 'text', id: 'aws_sm_prefix', placeholder: 'windmill/', disabled }} bind:value={$values['secret_backend'].prefix} />
				</div>
				<div class="flex flex-col gap-1">
					<label for="aws_sm_endpoint_url" class="block text-xs font-semibold text-emphasis">Endpoint URL (optional)</label>
					<span class="text-2xs text-secondary">Custom endpoint for LocalStack or other compatible services</span>
					<TextInput inputProps={{ type: 'text', id: 'aws_sm_endpoint_url', placeholder: 'http://localhost:4566', disabled }} bind:value={$values['secret_backend'].endpoint_url} />
				</div>
			</div>
			<div class="flex flex-col gap-4 pt-4 border-t">
				<Button unifiedSize="md" variant="accent" onclick={testAwsSmConnection} disabled={disabled || !isAwsSmConfigValid() || testingAwsSmConnection} loading={testingAwsSmConnection} startIcon={{ icon: Server }}>Test Connection</Button>
				<div class="flex flex-col gap-4 pt-4 border-t">
					<span class="block text-xs font-semibold text-emphasis">Secret Migration</span>
					<span class="text-2xs text-secondary">Original values are NOT deleted to allow for rollback.</span>
					<div class="flex gap-4">
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2"><Database size={16} /><ArrowRight size={16} /><Cloud size={16} /></div>
							<p class="text-xs font-medium mb-2">Database → AWS Secrets Manager</p>
							<Button unifiedSize="sm" variant="default" onclick={() => (migrateToAwsSmModalOpen = true)} disabled={disabled || !isAwsSmConfigValid() || migratingToAwsSm} startIcon={{ icon: ArrowRight }}>Migrate to AWS SM</Button>
						</div>
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2"><Cloud size={16} /><ArrowLeft size={16} /><Database size={16} /></div>
							<p class="text-xs font-medium mb-2">AWS Secrets Manager → Database</p>
							<Button unifiedSize="sm" variant="default" onclick={() => (migrateFromAwsSmModalOpen = true)} disabled={disabled || !isAwsSmConfigValid() || migratingFromAwsSm} startIcon={{ icon: ArrowLeft }}>Migrate to Database</Button>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<ConfirmationModal title="Migrate to AWS Secrets Manager" confirmationText="Migrate" open={migrateToAwsSmModalOpen} loading={migratingToAwsSm} type="reload" onCanceled={() => { migrateToAwsSmModalOpen = false }} onConfirmed={migrateSecretsToAwsSm}>
	{#snippet children()}<div class="flex flex-col gap-2"><p>This will copy all secrets from the database to AWS Secrets Manager.</p><p class="text-yellow-600 dark:text-yellow-400 text-sm">Database values are NOT deleted automatically.</p></div>{/snippet}
</ConfirmationModal>
<ConfirmationModal title="Migrate to Database" confirmationText="Migrate" open={migrateFromAwsSmModalOpen} loading={migratingFromAwsSm} type="reload" onCanceled={() => { migrateFromAwsSmModalOpen = false }} onConfirmed={migrateSecretsFromAwsSm}>
	{#snippet children()}<div class="flex flex-col gap-2"><p>This will copy all secrets from AWS Secrets Manager back to the database.</p><p class="text-yellow-600 dark:text-yellow-400 text-sm">AWS Secrets Manager values are NOT deleted automatically.</p></div>{/snippet}
</ConfirmationModal>
<ConfirmationModal title="Migrate to Azure Key Vault" confirmationText="Migrate" open={migrateToAzureKvModalOpen} loading={migratingToAzureKv} type="reload" onCanceled={() => { migrateToAzureKvModalOpen = false }} onConfirmed={migrateSecretsToAzureKv}>
	{#snippet children()}<div class="flex flex-col gap-2"><p>This will copy all secrets to Azure Key Vault.</p><p class="text-yellow-600 dark:text-yellow-400 text-sm">Database values are NOT deleted automatically.</p></div>{/snippet}
</ConfirmationModal>
<ConfirmationModal title="Migrate to Database" confirmationText="Migrate" open={migrateFromAzureKvModalOpen} loading={migratingFromAzureKv} type="reload" onCanceled={() => { migrateFromAzureKvModalOpen = false }} onConfirmed={migrateSecretsFromAzureKv}>
	{#snippet children()}<div class="flex flex-col gap-2"><p>This will copy all secrets from Azure Key Vault back to the database.</p></div>{/snippet}
</ConfirmationModal>
<ConfirmationModal title="Migrate to Vault" confirmationText="Migrate" open={migrateToVaultModalOpen} loading={migratingToVault} type="reload" onCanceled={() => { migrateToVaultModalOpen = false }} onConfirmed={migrateSecretsToVault}>
	{#snippet children()}<div class="flex flex-col gap-2"><p>This will copy all secrets to HashiCorp Vault.</p><p class="text-yellow-600 dark:text-yellow-400 text-sm">Database values are NOT deleted automatically.</p></div>{/snippet}
</ConfirmationModal>
<ConfirmationModal title="Migrate to Database" confirmationText="Migrate" open={migrateToDatabaseModalOpen} loading={migratingToDatabase} type="reload" onCanceled={() => { migrateToDatabaseModalOpen = false }} onConfirmed={migrateSecretsToDatabase}>
	{#snippet children()}<div class="flex flex-col gap-2"><p>This will copy all secrets from Vault back to the database.</p></div>{/snippet}
</ConfirmationModal>
