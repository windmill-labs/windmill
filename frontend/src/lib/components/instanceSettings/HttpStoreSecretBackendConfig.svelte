<script lang="ts">
	import { Button } from '$lib/components/common'
	import Password from '../Password.svelte'
	import { SettingService, type HttpStoreSettings } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import TextInput from '../text_input/TextInput.svelte'
	import Toggle from '../Toggle.svelte'
	import { ArrowLeft, Database, Globe, Server } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import StringMapInput from './StringMapInput.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
		reportMigrationFailures: (context: string, report: any) => void
	}

	let { values, disabled = false, reportMigrationFailures }: Props = $props()

	let cfg = $derived($values['secret_backend'] as HttpStoreSettings & { type: 'HttpStore' })

	let probeWorkspace = $state('')
	let probePath = $state('')
	let testing = $state(false)
	let migrating = $state(false)
	let migrateModalOpen = $state(false)

	const EXAMPLE_PATH = 'u/alice/db_password'
	let exampleKey = $derived.by(() => {
		switch (cfg.path_encoding ?? 'url') {
			case 'double_dash':
				return EXAMPLE_PATH.replaceAll('/', '--')
			case 'raw':
				return EXAMPLE_PATH
			default:
				return encodeURIComponent(EXAMPLE_PATH)
		}
	})

	function setAuthType(type: string | undefined) {
		if (!type || type === cfg.auth?.type) return
		const auth =
			type === 'StaticHeader'
				? { type, header_name: 'Authorization', header_value: '' }
				: type === 'BearerTokenFile'
					? { type, token_path: '' }
					: type === 'ClientCertificate'
						? { type, cert_path: '', key_path: '' }
						: { type: 'None' }
		$values['secret_backend'] = { ...cfg, auth }
	}

	function optionalNumber(v: string | number | undefined): number | undefined {
		return v === '' || v == null ? undefined : Number(v)
	}

	// The form leaves empty strings behind; the backend reads an absent optional as its default.
	function getSettings(): HttpStoreSettings {
		const { type: _type, ...settings } = cfg
		const clean = (o: Record<string, any>) =>
			Object.fromEntries(Object.entries(o).filter(([, v]) => v !== '' && v != null))
		return {
			...(clean(settings) as HttpStoreSettings),
			read: clean(settings.read) as HttpStoreSettings['read'],
			auth: clean(settings.auth) as HttpStoreSettings['auth']
		}
	}

	function isConfigValid(): boolean {
		return !!cfg.base_url?.trim() && !!cfg.read?.path?.trim()
	}

	async function testConnection() {
		testing = true
		try {
			const msg = await SettingService.testHttpStoreBackend({
				requestBody: {
					settings: getSettings(),
					workspace_id: probeWorkspace.trim(),
					path: probePath.trim()
				}
			})
			sendUserToast(msg)
		} catch (error: any) {
			sendUserToast('Failed: ' + error.message, true)
		} finally {
			testing = false
		}
	}

	async function migrateToDatabase() {
		migrating = true
		try {
			const report = await SettingService.migrateSecretsFromHttpStore({
				requestBody: getSettings()
			})
			if (report.failed_count > 0) {
				reportMigrationFailures('HTTP store->DB migration', report)
			} else {
				sendUserToast(
					`Migrated ${report.migrated_count}/${report.total_secrets} secrets to database`
				)
			}
		} catch (error: any) {
			sendUserToast('Failed: ' + error.message, true)
		} finally {
			migrating = false
			migrateModalOpen = false
		}
	}
</script>

<div class="space-y-4 p-4 border rounded-lg">
	<div class="flex items-center gap-2 mb-4">
		<Globe class="text-primary" size={20} />
		<div>
			<p class="text-sm font-medium text-emphasis">Generic HTTP store configuration</p>
			<p class="text-xs text-secondary"
				>Read secrets from any secret store with an HTTP API. Windmill only reads: values are
				managed in the store. To reference a secret, save a secret variable with an empty value, and
				its path maps to a key in the store. Secrets already encrypted in the database keep working.</p
			>
		</div>
	</div>

	<div class="grid grid-cols-1 gap-4">
		<span class="block text-xs font-semibold text-emphasis">Endpoint</span>
		<div class="flex flex-col gap-1">
			<label for="http_store_base_url" class="block text-xs font-semibold text-emphasis"
				>Base URL</label
			>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'http_store_base_url',
					placeholder: 'https://secrets.example.internal',
					disabled
				}}
				bind:value={$values['secret_backend'].base_url}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="http_store_ca" class="block text-xs font-semibold text-emphasis"
				>CA bundle path (optional)</label
			>
			<span class="text-2xs text-secondary"
				>PEM file of extra CAs trusted for the store's server certificate, re-read when it changes</span
			>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'http_store_ca',
					placeholder: '/etc/ssl/certs/internal-ca.pem',
					disabled
				}}
				bind:value={$values['secret_backend'].ca_bundle_path}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="http_store_timeout" class="block text-xs font-semibold text-emphasis"
				>Timeout in seconds (optional)</label
			>
			<TextInput
				inputProps={{ type: 'number', id: 'http_store_timeout', placeholder: '10', disabled }}
				bind:value={
					() => $values['secret_backend'].timeout_secs ?? '',
					(v) => ($values['secret_backend'].timeout_secs = optionalNumber(v))
				}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<Toggle
				id="http_store_skip_ssl_verify"
				{disabled}
				bind:checked={$values['secret_backend'].skip_ssl_verify}
				size="xs"
				options={{ right: 'Skip TLS certificate verification' }}
			/>
			<span class="text-2xs text-secondary"
				>Only enable for self-signed certificates in development.</span
			>
		</div>

		<span class="block text-xs font-semibold text-emphasis pt-4 border-t">Read request</span>
		<div class="flex flex-col gap-1">
			<label for="http_store_path" class="block text-xs font-semibold text-emphasis"
				>Path template</label
			>
			<span class="text-2xs text-secondary"
				>Appended to the base URL. <code>{'{workspace}'}</code> and <code>{'{path}'}</code> must each
				appear here or in a query value, so that workspaces cannot read each other's secrets.</span
			>
			<TextInput
				inputProps={{
					type: 'text',
					id: 'http_store_path',
					placeholder: '/v1/secrets/{workspace}/{path}',
					disabled
				}}
				bind:value={$values['secret_backend'].read.path}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<span class="block text-xs font-semibold text-emphasis">Query parameters (optional)</span>
			<StringMapInput
				value={cfg.read?.query}
				onChange={(query) => ($values['secret_backend'].read.query = query)}
				valuePlaceholder="value, may use {'{workspace}'} or {'{path}'}"
				addLabel="Add parameter"
				{disabled}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<span class="block text-xs font-semibold text-emphasis">Path encoding</span>
			<span class="text-2xs text-secondary"
				>How a variable path becomes the store key: <code>{EXAMPLE_PATH}</code> →
				<code>{exampleKey}</code></span
			>
			<ToggleButtonGroup
				selected={cfg.path_encoding ?? 'url'}
				onSelected={(v) => ($values['secret_backend'].path_encoding = v)}
				{disabled}
			>
				{#snippet children({ item })}
					<ToggleButton value="url" label="URL-encoded" {item} />
					<ToggleButton
						value="double_dash"
						label="Double dash"
						tooltip="For stores whose keys cannot contain '/'. Paths with '--', or with a segment starting or ending with '-', are refused."
						{item}
					/>
					<ToggleButton value="raw" label="Raw" tooltip="Path segments as URL segments" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>
		<div class="flex flex-col gap-1">
			<label for="http_store_pointer" class="block text-xs font-semibold text-emphasis"
				>Value pointer (optional)</label
			>
			<span class="text-2xs text-secondary"
				>JSON pointer to the value in the response, e.g. <code>/data/value</code>. Empty means the
				whole body is the value.</span
			>
			<TextInput
				inputProps={{ type: 'text', id: 'http_store_pointer', placeholder: '/value', disabled }}
				bind:value={$values['secret_backend'].read.value_pointer}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<span class="block text-xs font-semibold text-emphasis">Value encoding</span>
			<ToggleButtonGroup
				selected={cfg.read?.value_encoding ?? 'plain'}
				onSelected={(v) => ($values['secret_backend'].read.value_encoding = v)}
				{disabled}
			>
				{#snippet children({ item })}
					<ToggleButton value="plain" label="Plain" {item} />
					<ToggleButton value="base64" label="Base64" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</div>

		<span class="block text-xs font-semibold text-emphasis pt-4 border-t">Authentication</span>
		<span class="text-2xs text-secondary"
			>Credential files must be mounted on servers and workers; agent workers never read them.</span
		>
		<ToggleButtonGroup
			selected={cfg.auth?.type ?? 'None'}
			onSelected={(v) => setAuthType(v)}
			{disabled}
		>
			{#snippet children({ item })}
				<ToggleButton value="None" label="None" {item} />
				<ToggleButton value="StaticHeader" label="Static header" {item} />
				<ToggleButton
					value="BearerTokenFile"
					label="Token file"
					tooltip="A bearer token read from a file, such as a Kubernetes projected service account token"
					{item}
				/>
				<ToggleButton value="ClientCertificate" label="Client certificate" {item} />
			{/snippet}
		</ToggleButtonGroup>
		{#if cfg.auth?.type === 'StaticHeader'}
			<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
				<label for="http_store_header_name" class="block text-xs font-semibold text-emphasis"
					>Header name</label
				>
				<TextInput
					inputProps={{ type: 'text', id: 'http_store_header_name', disabled }}
					bind:value={$values['secret_backend'].auth.header_name}
				/>
				<span class="block text-xs font-semibold text-emphasis">Header value</span>
				<Password bind:password={$values['secret_backend'].auth.header_value} small {disabled} />
			</div>
		{:else if cfg.auth?.type === 'BearerTokenFile'}
			<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
				<label for="http_store_token_path" class="block text-xs font-semibold text-emphasis"
					>Token file path</label
				>
				<TextInput
					inputProps={{
						type: 'text',
						id: 'http_store_token_path',
						placeholder: '/var/run/secrets/tokens/secret-store',
						disabled
					}}
					bind:value={$values['secret_backend'].auth.token_path}
				/>
				<label for="http_store_token_header" class="block text-xs font-semibold text-emphasis"
					>Header name and prefix (optional)</label
				>
				<div class="flex gap-2">
					<TextInput
						inputProps={{
							type: 'text',
							id: 'http_store_token_header',
							placeholder: 'Authorization',
							disabled
						}}
						bind:value={$values['secret_backend'].auth.header_name}
					/>
					<TextInput
						inputProps={{ type: 'text', placeholder: 'Bearer ', disabled }}
						bind:value={$values['secret_backend'].auth.prefix}
					/>
				</div>
				<label for="http_store_token_reload" class="block text-xs font-semibold text-emphasis"
					>Reload interval in seconds (optional)</label
				>
				<span class="text-2xs text-secondary"
					>The file is re-read at this interval, and sooner after a 401 (default 60)</span
				>
				<TextInput
					inputProps={{
						type: 'number',
						id: 'http_store_token_reload',
						placeholder: '60',
						disabled
					}}
					bind:value={
						() => $values['secret_backend'].auth.reload_interval_secs ?? '',
						(v) => ($values['secret_backend'].auth.reload_interval_secs = optionalNumber(v))
					}
				/>
			</div>
		{:else if cfg.auth?.type === 'ClientCertificate'}
			<div class="flex flex-col gap-2 p-3 bg-surface-secondary rounded-lg">
				<label for="http_store_cert_path" class="block text-xs font-semibold text-emphasis"
					>Certificate path</label
				>
				<span class="text-2xs text-secondary"
					>PEM file with the client certificate, optionally followed by its intermediates</span
				>
				<TextInput
					inputProps={{
						type: 'text',
						id: 'http_store_cert_path',
						placeholder: '/var/run/identity/chain.pem',
						disabled
					}}
					bind:value={$values['secret_backend'].auth.cert_path}
				/>
				<label for="http_store_key_path" class="block text-xs font-semibold text-emphasis"
					>Private key path</label
				>
				<TextInput
					inputProps={{
						type: 'text',
						id: 'http_store_key_path',
						placeholder: '/var/run/identity/private.pem',
						disabled
					}}
					bind:value={$values['secret_backend'].auth.key_path}
				/>
				<label for="http_store_cert_reload" class="block text-xs font-semibold text-emphasis"
					>Reload interval in seconds (optional)</label
				>
				<span class="text-2xs text-secondary"
					>The files are re-read at this interval, and sooner after a failed connection (default
					300). A new certificate is only used once its key matches.</span
				>
				<TextInput
					inputProps={{
						type: 'number',
						id: 'http_store_cert_reload',
						placeholder: '300',
						disabled
					}}
					bind:value={
						() => $values['secret_backend'].auth.reload_interval_secs ?? '',
						(v) => ($values['secret_backend'].auth.reload_interval_secs = optionalNumber(v))
					}
				/>
			</div>
		{/if}
		<div class="flex flex-col gap-1">
			<span class="block text-xs font-semibold text-emphasis">Extra headers (optional)</span>
			<StringMapInput
				value={cfg.extra_headers}
				onChange={(headers) => ($values['secret_backend'].extra_headers = headers)}
				keyPlaceholder="X-Client"
				addLabel="Add header"
				{disabled}
			/>
		</div>
	</div>

	<div class="flex flex-col gap-4 pt-4 border-t">
		<span class="block text-xs font-semibold text-emphasis">Test connection</span>
		<span class="text-2xs text-secondary"
			>Reads one secret that exists in the store, given as the workspace and variable path that map
			to it. The value is not shown.</span
		>
		<div class="flex gap-2">
			<TextInput
				inputProps={{ placeholder: 'workspace id', disabled }}
				bind:value={probeWorkspace}
			/>
			<TextInput
				inputProps={{ placeholder: 'u/alice/db_password', disabled }}
				bind:value={probePath}
			/>
		</div>
		<div>
			<Button
				unifiedSize="md"
				variant="accent"
				onclick={testConnection}
				disabled={disabled ||
					!isConfigValid() ||
					!probeWorkspace.trim() ||
					!probePath.trim() ||
					testing}
				loading={testing}
				startIcon={{ icon: Server }}>Test connection</Button
			>
		</div>
		<div class="flex flex-col gap-2 pt-4 border-t">
			<span class="block text-xs font-semibold text-emphasis">Secret migration</span>
			<div class="p-3 border rounded-lg">
				<div class="flex items-center gap-2 mb-2"
					><Globe size={16} /><ArrowLeft size={16} /><Database size={16} /></div
				>
				<p class="text-xs font-medium mb-2">HTTP store → Database</p>
				<p class="text-2xs text-secondary mb-2"
					>Copies the value of every reference into the database. Windmill cannot write to the
					store, so there is no migration the other way.</p
				>
				<Button
					unifiedSize="sm"
					variant="default"
					onclick={() => (migrateModalOpen = true)}
					disabled={disabled || !isConfigValid() || migrating}
					startIcon={{ icon: ArrowLeft }}>Migrate to database</Button
				>
			</div>
		</div>
	</div>
</div>

<ConfirmationModal
	title="Migrate to database"
	confirmationText="Migrate"
	open={migrateModalOpen}
	loading={migrating}
	type="reload"
	onCanceled={() => {
		migrateModalOpen = false
	}}
	onConfirmed={migrateToDatabase}
>
	{#snippet children()}<div class="flex flex-col gap-2"
			><p>This will copy the value of every HTTP store reference into the database.</p><p
				class="text-yellow-600 dark:text-yellow-400 text-sm">The values stay in the store.</p
			></div
		>{/snippet}
</ConfirmationModal>
