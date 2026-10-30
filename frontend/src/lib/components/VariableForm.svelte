<script lang="ts">
	import Path from './Path.svelte'
	import LabelsInput from './LabelsInput.svelte'
	import Toggle from './Toggle.svelte'
	import Alert from './common/alert/Alert.svelte'
	import { Button, Section } from './common'
	import Tooltip from './Tooltip.svelte'
	import Label from './Label.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { Loader2, RotateCcw } from 'lucide-svelte'
	import autosize from '$lib/autosize'
	import { type UserExt, userWorkspaces } from '$lib/stores'
	import { capabilitiesIn } from '$lib/roleCapabilities'
	import { isOwner } from '$lib/utils'
	import { isEncryptedDraftValue } from '$lib/encryptedDraft'
	import EncryptedDraftField from './EncryptedDraftField.svelte'
	import DateTimeInput from './DateTimeInput.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import { formatVaultRef, isExternalRef, parseVaultRef } from '$lib/externalSecretRef'
	import { VariableService } from '$lib/gen'
	import { resource } from 'runed'

	const operatingWorkspace = useOperatingWorkspace()

	interface Variable {
		value: string
		is_secret: boolean
		description: string
	}

	interface Props {
		path: string
		initialPath: string
		pathError: string
		variable: Variable
		labels: string[] | undefined
		wsSpecific: boolean
		valueValidUntil: string | undefined
		deployTo: string | undefined
		can_write: boolean
		edit: boolean
		onLoadSecret?: () => void
		/** Fires after the user flips the Secret toggle, with its new state. */
		onSecretChange?: (isSecret: boolean) => void
		/** Whether the deployed variable being edited is secret. */
		deployedSecret?: boolean
		/** Whether the deployed variable being edited is an external reference. */
		deployedRef?: boolean
		/** Workspace the path is validated against; defaults to the nav workspace. */
		workspace?: string | undefined
		/** The user acting in `workspace`, resolved by the editor above. `undefined` while
		 * `null` while that lookup is pending or after it failed: every check below then
		 * refuses, rather than answering with the navigation user's rights in another
		 * workspace. */
		actingUser: UserExt | null
		/** Linked to an OAuth account, whose token Windmill refreshes on its own. */
		oauthLinked?: boolean
	}

	let {
		path = $bindable(),
		initialPath,
		pathError = $bindable(),
		variable = $bindable(),
		labels = $bindable(),
		wsSpecific = $bindable(),
		valueValidUntil = $bindable(),
		deployTo,
		can_write,
		edit,
		onLoadSecret,
		onSecretChange,
		deployedSecret = false,
		deployedRef = false,
		workspace = undefined,
		actingUser,
		oauthLinked = false
	}: Props = $props()

	let ws = $derived(workspace ?? $operatingWorkspace)
	let caps = $derived(
		capabilitiesIn(
			actingUser ?? undefined,
			$userWorkspaces.find((w) => w.id === ws)
		)
	)

	// Enabling seeds a date: an empty DateTimeInput still shows 12:00, which reads as set.
	const DEFAULT_EXPIRY_DAYS = 90

	function defaultValueExpiry(): string {
		const d = new Date()
		d.setDate(d.getDate() + DEFAULT_EXPIRY_DAYS)
		return d.toISOString()
	}

	// Loading the deployed secret overwrites the draft row this form shares with the AI
	// chat, so every path that would trigger it has to be blocked while that row stages a
	// value — otherwise the staged one is replaced and the next deploy carries the old one.
	// '' is the sentinel for "stages nothing", matching the deploy bodies.
	let hasStagedValue = $derived(variable.value !== '')

	// Un-securing has to send the plaintext, and fetching it silently on the toggle would
	// reveal the secret without the explicit, audited "Load secret value" action.
	let unsecureNeedsLoad = $derived(edit && deployedSecret && variable.is_secret && !hasStagedValue)

	// The API refuses anyone else creating, repointing or removing a reference.
	let canEditRef = $derived(can_write && !!actingUser?.is_admin)

	const MAX_VARIABLE_LENGTH = 10000

	let externalRef = $derived(variable.is_secret && isExternalRef(variable.value))
	let vaultRef = $derived(parseVaultRef(variable.value))
	let kind = $derived(externalRef ? 'ref' : variable.is_secret ? 'secret' : 'plain')
	let secretLocked = $derived(
		edit && (!actingUser || !caps.code || isEncryptedDraftValue(variable.value))
	)

	function setKind(next: string | undefined) {
		if (!next || next === kind) return
		if (kind === 'ref') variable.value = ''
		if (variable.is_secret !== (next !== 'plain')) {
			variable.is_secret = next !== 'plain'
			onSecretChange?.(variable.is_secret)
		}
		if (next === 'ref') variable.value = formatVaultRef('', '')
	}
	let allowedPrefixes = resource(
		() => (externalRef && canEditRef ? ws : undefined),
		async (ws) =>
			ws ? await VariableService.listExternalSecretPrefixes({ workspace: ws }) : undefined
	)
	let pathPlaceholder = $derived(
		(allowedPrefixes.current?.[0]?.replace(/^vault:\/\//, '').replace(/\/*$/, '') ??
			'secret/team') + '/app'
	)

	let editorKind: 'plain' | 'json' | 'yaml' = $state('plain')
	let editor: any = $state(undefined)

	export function setCode(value: string) {
		editor?.setCode(value)
	}
</script>

<div class="flex flex-col gap-1">
	<label for="path" class="text-xs font-semibold text-emphasis">Path</label>
	<Path
		disabled={initialPath != '' && !isOwner(initialPath, actingUser ?? undefined, ws)}
		bind:error={pathError}
		bind:path
		{initialPath}
		namePlaceholder="variable"
		kind="variable"
		workspaceOverride={workspace}
		{actingUser}
	/>
	<LabelsInput bind:labels />
</div>
<div class="flex flex-col gap-1">
	<span class="text-xs font-semibold text-emphasis">Type</span>
	<div class="flex flex-row items-center gap-2">
		<!-- An `$encrypted:` value is only redeemable while the variable stays secret — the
		deploy endpoints decrypt the marker inside their `is_secret` branch and store it
		verbatim otherwise — so un-securing one has to be unreachable until it is Reset. -->
		<ToggleButtonGroup selected={kind} onSelected={setKind}>
			{#snippet children({ item })}
				<ToggleButton
					value="plain"
					label="Non-secret"
					{item}
					disabled={kind !== 'plain' &&
						(secretLocked || unsecureNeedsLoad || (edit && externalRef))}
				/>
				<ToggleButton
					value="secret"
					label="Secret"
					{item}
					disabled={kind === 'plain'
						? secretLocked
						: kind === 'ref' && (!canEditRef || isEncryptedDraftValue(variable.value))}
				/>
				{#if canEditRef || externalRef}
					<ToggleButton
						value="ref"
						label="Vault reference"
						tooltip="Read the value from an existing secret in HashiCorp Vault"
						{item}
						disabled={kind !== 'ref' &&
							(!canEditRef ||
								isEncryptedDraftValue(variable.value) ||
								(kind === 'plain' && secretLocked))}
					/>
				{/if}
			{/snippet}
		</ToggleButtonGroup>
	</div>
	{#if externalRef && can_write && !canEditRef}
		<span class="text-xs text-secondary">
			Only workspace admins can create or change a reference
		</span>
	{/if}
	{#if unsecureNeedsLoad && can_write && (deployedRef || caps.code)}
		<span class="text-xs text-secondary">
			{deployedRef
				? 'Enter a value to replace the Vault reference'
				: 'Load the secret value or enter a new one to make it non-secret'}
		</span>
	{/if}
	{#if externalRef}
		<Alert type="info" title="Read from Vault on each access">
			Only the reference is stored in Windmill. The value is read from HashiCorp Vault each time the
			variable is read, and every read generates an audit log whose operation name is:
			variables.decrypt_secret
		</Alert>
	{:else if variable.is_secret}
		<Alert type="info" title="Audit log for each access">
			Every secret is encrypted at rest and in transit with a key specific to this workspace. In
			addition, any read of a secret variable generates an audit log whose operation name is:
			variables.decrypt_secret
		</Alert>
	{/if}
</div>

{#if deployTo}
	<Label
		label="Workspace specific"
		tooltip="Prevents this variable from being deployed to prod/staging. May have been enabled automatically because a workspace-specific resource references this variable via $var:. Disabling this toggle does not retroactively un-mark the resource that referenced it."
	>
		<Toggle bind:checked={wsSpecific} />
	</Label>
{/if}

{#if externalRef}
	<div class="flex flex-col gap-2">
		<div class="flex flex-col gap-1">
			<label for="external-ref-path" class="text-xs font-semibold text-emphasis">Vault path</label>
			{#if allowedPrefixes.current?.length}
				<span class="text-xs text-secondary">
					Under <code
						>{allowedPrefixes.current
							.map((p) => p.replace(/^vault:\/\//, '').replace(/\/*$/, '/**'))
							.join(', ')}</code
					>
				</span>
			{:else if allowedPrefixes.current}
				<Alert type="warning" size="xs" title="No Vault paths allowed for this workspace">
					Ask a superadmin to allow one in the instance settings.
				</Alert>
			{/if}
			<TextInput
				inputProps={{
					id: 'external-ref-path',
					placeholder: pathPlaceholder,
					disabled: !canEditRef
				}}
				bind:value={
					() => vaultRef.path, (v) => (variable.value = formatVaultRef(v ?? '', vaultRef.key))
				}
			/>
		</div>
		<div class="flex flex-col gap-1">
			<label for="external-ref-key" class="text-xs font-semibold text-emphasis">
				Key (optional)
			</label>
			<span class="text-xs text-secondary">
				The entry to read from the secret, like <code>token</code>. Leave empty to get all its
				entries as a JSON object.
			</span>
			<TextInput
				inputProps={{ id: 'external-ref-key', placeholder: 'token', disabled: !canEditRef }}
				bind:value={
					() => vaultRef.key, (v) => (variable.value = formatVaultRef(vaultRef.path, v ?? ''))
				}
			/>
		</div>
	</div>
{:else}
	<div class="flex flex-col gap-1">
		<label for="variable-value" class="flex flex-row justify-left items-center">
			<span class="text-xs font-semibold text-emphasis">Variable value&nbsp;</span>
			{#if !isEncryptedDraftValue(variable.value)}
				<span class="text-xs text-secondary font-normal">
					({variable.value.length}/{MAX_VARIABLE_LENGTH} characters)
				</span>
			{/if}
			{#if edit && variable.is_secret && !(deployedRef && !hasStagedValue)}
				<div class="ml-3"></div>
				{#if hasStagedValue}
					<!-- Clearing the staged value is the only way back to loading the deployed one;
				see `hasStagedValue`. An `$encrypted:` value additionally cannot be displayed. -->
					<Button
						size="xs"
						variant="default"
						startIcon={{ icon: RotateCcw }}
						disabled={!can_write}
						on:click={() => (variable.value = '')}
					>
						Reset
					</Button>
				{:else if !caps.code}
					<div class="p-2 border">Your role cannot load secret values</div>
				{:else}
					<Button size="xs" variant="default" on:click={() => onLoadSecret?.()}>
						Load secret value<Tooltip>Will generate an audit log</Tooltip>
					</Button>
				{/if}
			{/if}
		</label>
		<div>
			{#if isEncryptedDraftValue(variable.value)}
				<!-- The draft's secret value was encrypted server-side and can't be
			loaded back. Saving deploys the last saved value as-is; Reset clears
			it so a new secret can be typed. -->
				<EncryptedDraftField disabled={!can_write} onReset={() => (variable.value = '')} />
			{:else}
				<div class="flex flex-col gap-2">
					<ToggleButtonGroup bind:selected={editorKind}>
						{#snippet children({ item })}
							<ToggleButton value="plain" label="Plain" {item} />
							<ToggleButton value="json" label="Json" {item} />
							<ToggleButton value="yaml" label="YAML" {item} />
						{/snippet}
					</ToggleButtonGroup>
					{#if editorKind == 'plain'}
						<textarea
							disabled={!can_write}
							rows="4"
							use:autosize
							bind:value={variable.value}
							placeholder="Update variable value"
							id="variable-value"
						></textarea>
					{:else if editorKind == 'json'}
						<div class="border rounded mb-4 w-full">
							{#await import('$lib/components/SimpleEditor.svelte')}
								<Loader2 class="animate-spin" />
							{:then Module}
								<Module.default
									bind:this={editor}
									autoHeight
									lang="json"
									bind:code={variable.value}
									fixedOverflowWidgets={false}
									class="bg-surface-tertiary"
								/>
							{/await}
						</div>
					{:else if editorKind == 'yaml'}
						<div class="border rounded mb-4 w-full">
							{#await import('$lib/components/SimpleEditor.svelte')}
								<Loader2 class="animate-spin" />
							{:then Module}
								<Module.default
									bind:this={editor}
									autoHeight
									lang="yaml"
									bind:code={variable.value}
									fixedOverflowWidgets={false}
									class="bg-surface-tertiary"
								/>
							{/await}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}
<label class="flex flex-col gap-1">
	<span class="text-xs font-semibold text-emphasis">Description</span>
	<textarea rows="4" use:autosize bind:value={variable.description} placeholder="Used for X"
	></textarea>
</label>

{#if !oauthLinked}
	<Section label="Advanced" small collapsable initiallyCollapsed={!valueValidUntil}>
		<Label
			label="Value expiration"
			tooltip="When the value stored here stops working, such as the expiry date of an API key. Ahead of it (1 day by default), Windmill runs the workspace's variable expiration handler if one is set, so the value can be rotated. The variable itself is never deleted."
		>
			<Toggle
				size="sm"
				checked={Boolean(valueValidUntil)}
				disabled={!can_write}
				on:change={() => (valueValidUntil = valueValidUntil ? undefined : defaultValueExpiry())}
				options={{ right: 'Set a date on which this value expires' }}
			/>
			{#if valueValidUntil}
				<span class="text-xs font-semibold text-emphasis leading-none mt-2">Expires on</span>
				<!-- Unbound: DateTimeInput writes `null` when emptied, which reads as an edit,
			so empty states become `undefined`. Not `clearable`: the toggle means "no expiry". -->
				<DateTimeInput
					value={valueValidUntil}
					useDropdown
					disabled={!can_write}
					on:change={(e) => (valueValidUntil = e.detail ?? undefined)}
				/>
				<span class="text-2xs font-normal text-hint">
					{#if new Date(valueValidUntil).getTime() <= Date.now()}
						This date has passed. Set a new one once the value is rotated.
					{:else}
						Runs the workspace's <a
							href="/workspace_settings?tab=variable_expiration&workspace={ws}"
							>variable expiration handler</a
						>
						ahead of this date, if the workspace has one{path.startsWith('u/')
							? ' and does not skip u/ paths'
							: ''}.
					{/if}
				</span>
			{/if}
		</Label>
	</Section>
{/if}
