<script lang="ts">
	import { Button } from '$lib/components/common'
	import Password from '../Password.svelte'
	import { SettingService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import { Database, Lock, Server, ArrowLeft, ArrowRight } from 'lucide-svelte'
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

	// Initialize default values if not set
	$effect(() => {
		if (!$values['secret_backend']) {
			$values['secret_backend'] = { type: 'Database' }
		}
	})

	let selectedType: 'Database' | 'HashiCorpVault' = $derived(
		$values['secret_backend']?.type ?? 'Database'
	)

	// Derive auth method from current config
	// We check jwt_role === null because setAuthMethod explicitly sets jwt_role to null for token mode
	// and sets token to null for jwt mode. This allows empty token values while still tracking the selection.
	let authMethod: 'token' | 'jwt' = $derived.by(() => {
		const config = $values['secret_backend']
		if (!config || config.type !== 'HashiCorpVault') return 'jwt'
		// If jwt_role is explicitly null, we're in token mode; otherwise jwt mode
		return config.jwt_role === null ? 'token' : 'jwt'
	})

	let testingConnection = $state(false)
	let migratingToVault = $state(false)
	let migratingToDatabase = $state(false)
	let migrateToVaultModalOpen = $state(false)
	let migrateToDatabaseModalOpen = $state(false)

	// Check if Vault option should be disabled (non-EE)
	let vaultDisabled = $derived(!$enterpriseLicense)

	function setBackendType(type: string | undefined) {
		if (!type) return
		// Prevent selecting Vault in non-EE
		if (type === 'HashiCorpVault' && vaultDisabled) {
			return
		}
		if (type === 'Database') {
			$values['secret_backend'] = { type: 'Database' }
		} else if (type === 'HashiCorpVault') {
			$values['secret_backend'] = {
				type: 'HashiCorpVault',
				address: $values['secret_backend']?.address ?? '',
				mount_path: $values['secret_backend']?.mount_path ?? 'windmill',
				jwt_role: $values['secret_backend']?.jwt_role ?? 'windmill-secrets',
				namespace: $values['secret_backend']?.namespace ?? null,
				token: $values['secret_backend']?.token ?? null
			}
		}
	}

	function setAuthMethod(method: string | undefined) {
		if (
			!method ||
			!$values['secret_backend'] ||
			$values['secret_backend'].type !== 'HashiCorpVault'
		)
			return

		if (method === 'token') {
			// Clear JWT role when switching to token auth
			$values['secret_backend'] = {
				...$values['secret_backend'],
				jwt_role: null,
				token: $values['secret_backend'].token ?? ''
			}
		} else if (method === 'jwt') {
			// Clear token when switching to JWT auth
			$values['secret_backend'] = {
				...$values['secret_backend'],
				token: null,
				jwt_role: $values['secret_backend'].jwt_role ?? 'windmill-secrets'
			}
		}
	}

	function getVaultSettings() {
		return {
			address: $values['secret_backend'].address,
			mount_path: $values['secret_backend'].mount_path,
			jwt_role: $values['secret_backend'].jwt_role,
			namespace: $values['secret_backend'].namespace || undefined,
			token: $values['secret_backend'].token || undefined
		}
	}

	async function testVaultConnection() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') {
			return
		}

		testingConnection = true
		try {
			await SettingService.testSecretBackend({
				requestBody: getVaultSettings()
			})
			sendUserToast('Successfully connected to HashiCorp Vault')
		} catch (error: any) {
			sendUserToast('Failed to connect to Vault: ' + error.message, true)
		} finally {
			testingConnection = false
		}
	}

	async function migrateSecretsToVault() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') {
			return
		}

		migratingToVault = true
		try {
			const report = await SettingService.migrateSecretsToVault({
				requestBody: getVaultSettings()
			})
			if (report.failed_count > 0) {
				sendUserToast(
					`Migration completed with errors: ${report.migrated_count}/${report.total_secrets} secrets migrated, ${report.failed_count} failed`,
					true
				)
				console.error('Migration failures:', report.failures)
			} else {
				sendUserToast(
					`Successfully migrated ${report.migrated_count}/${report.total_secrets} secrets to Vault`
				)
			}
		} catch (error: any) {
			sendUserToast('Failed to migrate secrets to Vault: ' + error.message, true)
		} finally {
			migratingToVault = false
			migrateToVaultModalOpen = false
		}
	}

	async function migrateSecretsToDatabase() {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') {
			return
		}

		migratingToDatabase = true
		try {
			const report = await SettingService.migrateSecretsToDatabase({
				requestBody: getVaultSettings()
			})
			if (report.failed_count > 0) {
				sendUserToast(
					`Migration completed with errors: ${report.migrated_count}/${report.total_secrets} secrets migrated, ${report.failed_count} failed`,
					true
				)
				console.error('Migration failures:', report.failures)
			} else {
				sendUserToast(
					`Successfully migrated ${report.migrated_count}/${report.total_secrets} secrets to database`
				)
			}
		} catch (error: any) {
			sendUserToast('Failed to migrate secrets to database: ' + error.message, true)
		} finally {
			migratingToDatabase = false
			migrateToDatabaseModalOpen = false
		}
	}

	function isVaultConfigValid(): boolean {
		if (!$values['secret_backend'] || $values['secret_backend'].type !== 'HashiCorpVault') {
			return false
		}
		const hasAddress = $values['secret_backend'].address?.trim() !== ''
		const hasMountPath = $values['secret_backend'].mount_path?.trim() !== ''
		const hasToken = $values['secret_backend'].token?.trim()
		const hasJwtRole = $values['secret_backend'].jwt_role?.trim()

		// Must have address and mount path, plus either token OR jwt_role (not both)
		return hasAddress && hasMountPath && (hasToken || hasJwtRole)
	}

	// Get the base URL for JWKS endpoint instructions (from instance settings)
	let baseUrl = $derived($values['base_url'] ?? 'https://your-windmill-instance.com')
</script>

<div class="space-y-6">
	<!-- Backend Type Selector -->
	<div class="flex flex-col gap-2 mt-1">
		<ToggleButtonGroup selected={selectedType} onSelected={(v) => setBackendType(v)}>
			{#snippet children({ item: toggleButton })}
				<ToggleButton
					value="Database"
					label="Database"
					tooltip="Store secrets encrypted in the database (default)"
					item={toggleButton}
				/>
				<ToggleButton
					value="HashiCorpVault"
					label="HashiCorp Vault (Beta)"
					tooltip={vaultDisabled
						? 'HashiCorp Vault integration requires Enterprise Edition'
						: 'Store secrets in HashiCorp Vault (Beta feature)'}
					item={toggleButton}
					disabled={vaultDisabled}
				/>
			{/snippet}
		</ToggleButtonGroup>
		{#if vaultDisabled}
			<div class="flex items-center gap-1">
				<EEOnly>HashiCorp Vault integration requires Enterprise Edition</EEOnly>
			</div>
		{/if}
	</div>

	{#if selectedType === 'Database'}
		<div class="flex items-center gap-2 p-4 bg-surface-secondary rounded-lg">
			<Database class="text-primary" size={20} />
			<div>
				<p class="text-sm font-medium text-emphasis">Database Storage (Default)</p>
				<p class="text-xs text-secondary">
					Secrets are encrypted using workspace-specific keys and stored in the PostgreSQL database.
				</p>
			</div>
		</div>
	{:else if selectedType === 'HashiCorpVault'}
		<!-- Vault Configuration -->
		<div class="space-y-4 p-4 border rounded-lg">
			<div class="flex items-center gap-2 mb-4">
				<Lock class="text-primary" size={20} />
				<div>
					<p class="text-sm font-medium text-emphasis">
						HashiCorp Vault Configuration
						<span
							class="ml-2 px-1.5 py-0.5 text-2xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200 rounded"
							>Beta</span
						>
					</p>
					<p class="text-xs text-secondary">
						Store secrets in an external HashiCorp Vault instance.
					</p>
				</div>
			</div>

			<div class="grid grid-cols-1 gap-4">
				<div class="flex flex-col gap-1">
					<label for="vault_address" class="block text-xs font-semibold text-emphasis"
						>Vault Address</label
					>
					<TextInput
						inputProps={{
							type: 'text',
							id: 'vault_address',
							placeholder: 'https://vault.company.com:8200',
							disabled: disabled
						}}
						bind:value={$values['secret_backend'].address}
					/>
				</div>

				<div class="flex flex-col gap-1">
					<label for="vault_mount_path" class="block text-xs font-semibold text-emphasis"
						>KV Mount Path</label
					>
					<span class="text-2xs text-secondary">The KV v2 secrets engine mount path in Vault</span>
					<TextInput
						inputProps={{
							type: 'text',
							id: 'vault_mount_path',
							placeholder: 'windmill',
							disabled: disabled
						}}
						bind:value={$values['secret_backend'].mount_path}
					/>
				</div>

				<!-- Authentication Method Toggle -->
				<div class="flex flex-col gap-2">
					<span class="block text-xs font-semibold text-emphasis">Authentication Method</span>
					<ToggleButtonGroup selected={authMethod} onSelected={(v) => setAuthMethod(v)}>
						{#snippet children({ item: toggleButton })}
							<ToggleButton
								value="jwt"
								label="JWT Auth"
								tooltip="Authenticate using Windmill-signed JWTs (recommended for production)"
								item={toggleButton}
								{disabled}
							/>
							<ToggleButton
								value="token"
								label="Static Token"
								tooltip="Use a static Vault token (for testing/development)"
								item={toggleButton}
								{disabled}
							/>
						{/snippet}
					</ToggleButtonGroup>
				</div>

				{#if authMethod === 'token'}
					<div class="flex flex-col gap-1 p-3 bg-surface-secondary rounded-lg">
						<label for="vault_token" class="block text-xs font-semibold text-emphasis"
							>Vault Token</label
						>
						<span class="text-2xs text-secondary"
							>Static token for authentication. Recommended only for testing/development.</span
						>
						<Password bind:password={$values['secret_backend'].token} small {disabled} />
					</div>
				{:else}
					<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
						<label for="vault_jwt_role" class="block text-xs font-semibold text-emphasis"
							>JWT Auth Role</label
						>
						<span class="text-2xs text-secondary"
							>The JWT authentication role configured in Vault.</span
						>
						<TextInput
							inputProps={{
								type: 'text',
								id: 'vault_jwt_role',
								placeholder: 'windmill-secrets',
								disabled: disabled
							}}
							bind:value={$values['secret_backend'].jwt_role}
						/>

						<!-- Vault JWT Setup Instructions -->
						<details class="mt-2">
							<summary class="text-xs font-medium text-secondary cursor-pointer hover:text-primary"
								>Vault JWT Setup Instructions</summary
							>
							<div class="mt-2 p-2 bg-surface rounded text-2xs text-secondary space-y-2">
								<p>Configure Vault to accept JWTs from Windmill:</p>
								<div
									class="bg-gray-100 dark:bg-gray-800 p-2 rounded font-mono text-2xs overflow-x-auto"
								>
									<pre
										># Enable JWT auth method
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
  ttl="1h"</pre
									>
								</div>
								<p class="text-yellow-600 dark:text-yellow-400">
									Replace <code>windmill-secrets</code> with your role name if different.
								</p>
							</div>
						</details>
					</div>
				{/if}

				<div class="flex flex-col gap-1">
					<label for="vault_namespace" class="block text-xs font-semibold text-emphasis"
						>Namespace (optional)</label
					>
					<span class="text-2xs text-secondary"
						>Vault Enterprise namespace (leave empty if not using namespaces)</span
					>
					<TextInput
						inputProps={{
							type: 'text',
							id: 'vault_namespace',
							placeholder: 'admin/my-namespace',
							disabled: disabled
						}}
						bind:value={$values['secret_backend'].namespace}
					/>
				</div>
			</div>

			<!-- Action Buttons -->
			<div class="flex flex-col gap-4 pt-4 border-t">
				<div class="flex gap-2">
					<Button
						unifiedSize="md"
						variant="accent"
						onclick={testVaultConnection}
						disabled={disabled || !isVaultConfigValid() || testingConnection}
						loading={testingConnection}
						startIcon={{ icon: Server }}
					>
						Test Connection
					</Button>
				</div>

				<!-- Migration Section -->
				<div class="flex flex-col gap-4 pt-4 border-t">
					<span class="block text-xs font-semibold text-emphasis">Secret Migration</span>
					<span class="text-2xs text-secondary">
						Migrate secrets between the database and HashiCorp Vault. Original values are NOT
						deleted to allow for rollback.
					</span>

					<div class="flex gap-4">
						<!-- Database to Vault -->
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2">
								<Database size={16} />
								<ArrowRight size={16} />
								<Lock size={16} />
							</div>
							<p class="text-xs font-medium mb-2">Database → Vault</p>
							<p class="text-2xs text-secondary mb-3">
								Decrypt secrets from database and store in Vault
							</p>
							<Button
								unifiedSize="sm"
								variant="default"
								onclick={() => (migrateToVaultModalOpen = true)}
								disabled={disabled || !isVaultConfigValid() || migratingToVault}
								startIcon={{ icon: ArrowRight }}
							>
								Migrate to Vault
							</Button>
						</div>

						<!-- Vault to Database -->
						<div class="flex-1 p-3 border rounded-lg">
							<div class="flex items-center gap-2 mb-2">
								<Lock size={16} />
								<ArrowLeft size={16} />
								<Database size={16} />
							</div>
							<p class="text-xs font-medium mb-2">Vault → Database</p>
							<p class="text-2xs text-secondary mb-3">
								Read secrets from Vault and encrypt in database
							</p>
							<Button
								unifiedSize="sm"
								variant="default"
								onclick={() => (migrateToDatabaseModalOpen = true)}
								disabled={disabled || !isVaultConfigValid() || migratingToDatabase}
								startIcon={{ icon: ArrowLeft }}
							>
								Migrate to Database
							</Button>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<!-- Migrate to Vault Modal -->
<ConfirmationModal
	title="Migrate Secrets to Vault"
	confirmationText="Migrate"
	open={migrateToVaultModalOpen}
	loading={migratingToVault}
	type="reload"
	onCanceled={() => {
		migrateToVaultModalOpen = false
	}}
	onConfirmed={migrateSecretsToVault}
>
	{#snippet children()}
		<div class="flex flex-col gap-2">
			<p>
				This will migrate all existing secrets from the database to HashiCorp Vault. The process
				will:
			</p>
			<ol class="list-decimal list-inside text-sm space-y-1">
				<li>Read all encrypted secrets from the database</li>
				<li>Decrypt them using the workspace encryption keys</li>
				<li>Store them in HashiCorp Vault under the configured mount path</li>
			</ol>
			<p class="text-yellow-600 dark:text-yellow-400 text-sm mt-2">
				Note: Database values are NOT deleted automatically. You can manually clear them after
				verifying the migration was successful.
			</p>
			<p>Are you sure you want to proceed?</p>
		</div>
	{/snippet}
</ConfirmationModal>

<!-- Migrate to Database Modal -->
<ConfirmationModal
	title="Migrate Secrets to Database"
	confirmationText="Migrate"
	open={migrateToDatabaseModalOpen}
	loading={migratingToDatabase}
	type="reload"
	onCanceled={() => {
		migrateToDatabaseModalOpen = false
	}}
	onConfirmed={migrateSecretsToDatabase}
>
	{#snippet children()}
		<div class="flex flex-col gap-2">
			<p>
				This will migrate all secrets from HashiCorp Vault back to the database. The process will:
			</p>
			<ol class="list-decimal list-inside text-sm space-y-1">
				<li>List all secrets in Vault for each workspace</li>
				<li>Read each secret value from Vault</li>
				<li>Encrypt and store them in the database</li>
			</ol>
			<p class="text-yellow-600 dark:text-yellow-400 text-sm mt-2">
				Note: Vault values are NOT deleted automatically. Only secrets that already exist in the
				database will be updated.
			</p>
			<p>Are you sure you want to proceed?</p>
		</div>
	{/snippet}
</ConfirmationModal>
