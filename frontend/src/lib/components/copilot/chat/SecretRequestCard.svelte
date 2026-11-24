<script lang="ts">
	import { KeyRound, Lock, PlugZap, Rocket, ShieldCheck } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Password from '$lib/components/Password.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import Path from '$lib/components/Path.svelte'
	import CollapseLink from '$lib/components/CollapseLink.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { ResourceService } from '$lib/gen'
	import { resource as asyncResource } from 'runed'
	import {
		canTestUnsaved,
		testUnsavedResource,
		type ConnectionTestResult
	} from '$lib/components/resourceConnectionTest'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import { markdownProse } from '$lib/components/markdownProse'
	import LinkRenderer from './LinkRenderer.svelte'
	import {
		SecretCheckFailed,
		SecretDraftsInTheWay,
		connectionStringParts,
		fieldLabel,
		resourcePageUrl,
		secretFieldsOf,
		secretVariablePaths,
		variablePageUrl
	} from './secretRequest'
	import type { SecretRequestDisplay, SecretSubmission } from './shared'

	interface Props {
		request: SecretRequestDisplay
		/** Inert copies (an answered request in history) render the form but cannot save. */
		interactive?: boolean
		/** Saves what the user confirmed; the chat settles the card once it lands. Rejects with
		 * the refusal. */
		save?: (submission: SecretSubmission) => Promise<void>
		onDismiss?: () => void
	}

	let { request, interactive = true, save, onDismiss }: Props = $props()

	// The model writes the steps, so a prompt injection could put a `javascript:` link in them,
	// which LinkRenderer does not render as a link.
	const helpPlugins = [gfmPlugin(), { renderer: { a: LinkRenderer } }]
	const resource = $derived(request.resource)
	const secretFields = $derived(secretFieldsOf(request))
	// The chat and the secure input page test such a resource as they save it.
	const tested = $derived(!!resource && canTestUnsaved(resource.type, resource.value))
	// The resource type's own description of each field, as the resource editor shows it.
	const fieldDescriptions = asyncResource(
		() => ({ type: resource?.type, workspace: request.workspace }),
		async ({ type, workspace }): Promise<Record<string, string>> => {
			if (!type) return {}
			try {
				const rt = await ResourceService.getResourceType({ workspace, path: type })
				const properties = (rt.schema as { properties?: Record<string, { description?: string }> })
					?.properties
				return Object.fromEntries(
					Object.entries(properties ?? {}).flatMap(([k, p]) =>
						p?.description ? [[k, p.description]] : []
					)
				)
			} catch {
				return {}
			}
		}
	)

	let secrets = $state<Record<string, string>>({})
	// Only what the user changed; the rest stays as the AI prepared it.
	let fieldEdits = $state<Record<string, string>>({})
	let submitting = $state(false)
	const saving = $derived(submitting || !!request.saving)
	let saveError = $state<string | undefined>()
	// Saved, but the saved resource does not connect: the database's or provider's own words.
	let checkError = $state<string | undefined>()
	// Drafts the user already has where the card saves: the next save at that path replaces them.
	let draftsInTheWay = $state<{ path: string; message: string }>()

	// A taken path is the only error that still lets the user save: saving then updates what
	// is there.
	const TAKEN = $derived(
		resource
			? 'This path is already taken. Pick another or update the resource already there.'
			: 'This path is already taken. Pick another or update the secret already there.'
	)
	let pickedPath = $state<string | undefined>()
	let pathError = $state<string | undefined>()
	const savePath = $derived(pickedPath || request.path)
	const taken = $derived(pathError === TAKEN)
	const replaceDrafts = $derived(draftsInTheWay?.path === savePath)
	const variablePaths = $derived(secretVariablePaths(savePath, secretFields))
	// A type that also takes a connection string offers both; the AI only picks which opens.
	const offersString = $derived(resource?.connectionString !== undefined)
	let entry = $derived<'string' | 'fields'>(resource?.connectionString ? 'string' : 'fields')
	const useString = $derived(offersString && entry === 'string')
	let connectionString = $state('')
	const connection = $derived(
		useString && connectionString.trim() ? connectionStringParts(connectionString) : undefined
	)
	const connectionParts = $derived(connection && 'secret' in connection ? connection : undefined)

	const canSave = $derived(
		interactive &&
			(useString
				? !!connectionParts
				: secretFields.every((f) => (secrets[f.field] ?? '').length > 0)) &&
			!saving &&
			(!pathError || taken)
	)

	const testable = $derived(
		!!resource && !useString && canTestUnsaved(resource.type, resource.value)
	)
	const allSecrets = $derived(secretFields.every((f) => (secrets[f.field] ?? '').length > 0))
	function unsavedValue(): Record<string, unknown> {
		return { ...submittedFields(), ...$state.snapshot(secrets) }
	}
	let testing = $state(false)
	// Kept with the values it tested, so it disappears as soon as the user changes one.
	let unsavedTest = $state<{ of: string; result: ConnectionTestResult }>()
	const testResult = $derived(
		unsavedTest && unsavedTest.of === JSON.stringify(unsavedValue())
			? unsavedTest.result
			: undefined
	)

	async function testNow() {
		if (!resource || testing) return
		const value = unsavedValue()
		testing = true
		try {
			const result = await testUnsavedResource({
				workspace: request.workspace,
				resourceType: resource.type,
				value
			})
			if (result) unsavedTest = { of: JSON.stringify(value), result }
		} catch (e) {
			unsavedTest = {
				of: JSON.stringify(value),
				result: { ok: false, error: e instanceof Error ? e.message : String(e) }
			}
		} finally {
			testing = false
		}
	}

	/** Simple fields are editable; anything nested is shown as the AI wrote it. */
	function isEditable(v: unknown): v is string | number {
		return typeof v === 'string' || typeof v === 'number'
	}

	function submittedFields(): Record<string, unknown> | undefined {
		if (!resource) return undefined
		return Object.fromEntries(
			Object.entries(resource.value).map(([k, v]) => {
				if (!(k in fieldEdits)) return [k, v]
				const edited = fieldEdits[k]
				return [
					k,
					typeof v === 'number' && edited.trim() !== '' && !isNaN(+edited) ? +edited : edited
				]
			})
		)
	}

	async function submit() {
		if (!canSave || !save) return
		submitting = true
		saveError = undefined
		checkError = undefined
		try {
			await save({
				path: savePath,
				// The user asked the AI to replace what is at its path, which the path picker may not
				// show: it checks the resource path, not the variables beside it. Only that path.
				replace: taken || (!!request.replace && savePath === request.path),
				replaceDrafts,
				...(connectionParts
					? {
							secrets: { [secretFields[0].field]: connectionParts.secret },
							// Only what the string says, which is all the card shows.
							fields: connectionParts.fields
						}
					: { secrets: $state.snapshot(secrets), fields: submittedFields() })
			})
			secrets = {}
			connectionString = ''
		} catch (e) {
			if (e instanceof SecretDraftsInTheWay) draftsInTheWay = { path: savePath, message: e.message }
			else if (e instanceof SecretCheckFailed) checkError = e.message
			else saveError = e instanceof Error ? e.message : String(e)
		} finally {
			submitting = false
		}
	}

	function onSecretKeyDown(e: KeyboardEvent) {
		// Enter is a newline: a pasted private key keeps its line breaks.
		if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
			e.preventDefault()
			void submit()
		}
	}
</script>

{#snippet variableHint(lead: string, path: string, tail?: string)}
	<p class="flex items-start gap-1 text-2xs text-secondary">
		<Lock class="mt-0.5 h-3 w-3 shrink-0" />
		<span>{lead} <span class="font-mono">{path}</span>{tail ? `, ${tail}` : '.'}</span>
	</p>
{/snippet}

{#if request.saved}
	<!-- Settled, it reads as one more tool row in the turn, not a card. -->
	<div class="my-0.5 flex min-w-0 items-start gap-2 py-0.5 font-mono text-2xs leading-4">
		<ShieldCheck class="mt-px h-3.5 w-3.5 shrink-0 text-green-600 dark:text-green-400" />
		<span class="min-w-0 font-medium text-secondary">
			{#if resource}
				Saved {request.label} as a draft of the resource
				<a
					class="text-accent hover:underline"
					href={resourcePageUrl(request.saved.path, request.workspace)}>{request.saved.path}</a
				>
				· secrets in drafts of the encrypted variables
				{#each request.saved.variables as variable, i (variable)}
					{#if i > 0},
					{/if}
					<a class="text-accent hover:underline" href={variablePageUrl(variable, request.workspace)}
						>{variable}</a
					>
				{/each}
				{#if request.saved.check}
					· {request.saved.check.ok ? 'test passed' : 'test failed'}
				{/if}
			{:else}
				Saved {request.label} as a draft of secret variable
				<a
					class="text-accent hover:underline"
					href={variablePageUrl(request.saved.path, request.workspace)}>{request.saved.path}</a
				>
			{/if}
		</span>
	</div>
{:else if request.canceled}
	<div class="my-0.5 flex min-w-0 items-start gap-2 py-0.5 font-mono text-2xs leading-4">
		<Lock class="mt-px h-3.5 w-3.5 shrink-0 text-tertiary" />
		<span class="min-w-0 font-medium text-tertiary"
			>{resource ? 'Connection' : 'Secret request'} for {request.label} dismissed</span
		>
	</div>
{:else}
	<div
		class="overflow-hidden rounded-md border border-border-selected bg-surface-tertiary shadow-md"
		data-chat-keyboard-scope="secret-request"
	>
		<!-- The header is Windmill speaking, not the AI: the card is a form of the workspace that
		     the conversation happens to open. -->
		<div
			class="flex items-center gap-2 border-b border-border-light bg-surface-secondary px-3 py-2"
		>
			<Lock class="h-3.5 w-3.5 shrink-0 text-emphasis" />
			<span class="text-xs font-semibold text-emphasis">Secure input</span>
			<span class="text-2xs text-secondary"
				>· saved by Windmill as {resource ? 'a resource' : 'a secret variable'}, not sent to the AI</span
			>
		</div>

		<div class="flex flex-col gap-3 p-3">
			<div class="flex items-start gap-2">
				{#if resource}
					<div class="shrink-0 pt-0.5">
						<IconedResourceType name={resource.type} silent width="20px" height="20px" />
					</div>
				{:else}
					<KeyRound class="mt-0.5 h-4 w-4 shrink-0 text-secondary" />
				{/if}
				<div class="min-w-0 flex-1">
					<p class="text-xs font-semibold text-emphasis">
						{resource ? `Connect ${request.label}` : request.label}
					</p>
					<p class="text-2xs text-secondary">
						{#if request.description}
							{request.description}
						{:else if resource}
							Check what the AI prepared and enter the secrets.
						{:else}
							The AI needs this to continue.
						{/if}
					</p>
					{#if request.helpText}
						<div class="mt-1">
							<CollapseLink text="How to get these values" class="mb-0">
								<div class={markdownProse.xs}>
									<Markdown md={request.helpText} plugins={helpPlugins} />
								</div>
							</CollapseLink>
						</div>
					{/if}
				</div>
			</div>

			{#if offersString}
				<ToggleButtonGroup
					noWFull
					selected={entry}
					onSelected={(v) => (entry = v)}
					disabled={!interactive || saving}
				>
					{#snippet children({ item })}
						<ToggleButton value="string" label="Connection string" size="sm" {item} />
						<ToggleButton value="fields" label="Details" size="sm" {item} />
					{/snippet}
				</ToggleButtonGroup>
			{/if}

			{#if useString}
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-emphasis">Connection string</span>
					<Password
						bind:password={connectionString}
						placeholder="postgres://user:password@host:5432/dbname"
						disabled={!interactive || saving}
						autocomplete="off"
						error={saveError !== undefined || (connection !== undefined && !connectionParts)}
						onKeyDown={onSecretKeyDown}
					/>
					{#if connection && 'error' in connection}
						<p class="text-2xs text-red-600 dark:text-red-400">{connection.error}</p>
					{:else if connectionParts}
						{@const f = connectionParts.fields}
						<!-- Only what is not secret, so the user can check what they are connecting to. -->
						<p class="font-mono text-2xs text-secondary">
							{f.user} @ {f.host}:{f.port ?? 5432}/{f.dbname ?? ''} · sslmode {f.sslmode}
						</p>
					{/if}
					{@render variableHint(
						'The password is saved as the encrypted variable',
						variablePaths[0],
						'the rest in the resource.'
					)}
				</div>
			{:else}
				{#each secretFields as secretField, i (secretField.field)}
					<div class="flex flex-col gap-1">
						{#if resource}
							<span class="text-xs font-semibold text-emphasis">{secretField.label}</span>
							{#if fieldDescriptions.current?.[secretField.field]}
								<span class="text-xs text-secondary"
									>{fieldDescriptions.current[secretField.field]}</span
								>
							{/if}
						{/if}
						<Password
							bind:password={secrets[secretField.field]}
							placeholder="Paste the value here"
							disabled={!interactive || saving}
							autocomplete="off"
							error={saveError !== undefined}
							onKeyDown={onSecretKeyDown}
						/>
						{#if resource}
							{@render variableHint('Saved as the encrypted variable', variablePaths[i])}
						{/if}
					</div>
				{/each}
			{/if}

			{#if resource && !useString}
				{#each Object.entries(resource.value) as [key, fieldValue] (key)}
					<div class="flex flex-col gap-1">
						<span class="text-xs font-semibold text-emphasis">{fieldLabel(key)}</span>
						{#if fieldDescriptions.current?.[key]}
							<span class="text-xs text-secondary">{fieldDescriptions.current[key]}</span>
						{/if}
						{#if isEditable(fieldValue)}
							<TextInput
								size="sm"
								value={key in fieldEdits ? fieldEdits[key] : String(fieldValue)}
								inputProps={{
									disabled: !interactive || saving,
									oninput: (e: Event) => {
										fieldEdits[key] = (e.currentTarget as HTMLInputElement).value
									}
								}}
							/>
						{:else}
							<pre
								class="overflow-x-auto rounded-md bg-surface-secondary px-2 py-1 text-2xs text-secondary"
								>{JSON.stringify(fieldValue, null, 2)}</pre
							>
						{/if}
					</div>
				{/each}
			{/if}

			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">
					{resource ? 'Saved as a draft of the resource' : 'Saved as a draft of the secret variable'}
				</span>
				<Path
					kind={resource ? 'resource' : 'variable'}
					initialPath={request.path}
					bind:path={pickedPath}
					bind:error={pathError}
					checkInitialPathExistence
					takenError={TAKEN}
					workspaceOverride={request.workspace}
					disabled={!interactive || saving}
					autofocus={false}
					size="sm"
					warnOnRename={false}
				/>
			</div>

			<ul class="flex flex-col gap-1 text-2xs leading-4 text-secondary">
				<li class="flex items-start gap-1.5">
					<ShieldCheck class="mt-0.5 h-3 w-3 shrink-0 text-green-600 dark:text-green-400" />
					<span>
						Secret variables are where Windmill keeps secrets, encrypted.{resource
							? ' The resource holds the other fields and references them.'
							: ''}
					</span>
				</li>
				<li class="flex items-start gap-1.5">
					<Rocket class="mt-0.5 h-3 w-3 shrink-0 text-tertiary" />
					<span>Scripts and flows can use {resource ? 'them' : 'it'} once deployed.</span>
				</li>
				<li class="flex items-start gap-1.5">
					<ShieldCheck class="mt-0.5 h-3 w-3 shrink-0 text-green-600 dark:text-green-400" />
					<span>The AI never receives the values, only the path to reference them.</span>
				</li>
			</ul>

			{#if saveError}
				<p class="text-2xs text-red-600 dark:text-red-400">Could not save: {saveError}</p>
			{:else if replaceDrafts && draftsInTheWay}
				<p class="text-2xs text-orange-600 dark:text-orange-400">{draftsInTheWay.message}</p>
			{:else if checkError}
				<div class="flex flex-col gap-0.5 text-2xs">
					<p class="font-medium text-red-600 dark:text-red-400">
						Saved as a draft, but the connection test failed:
					</p>
					<p class="break-words font-mono text-red-600 dark:text-red-400">{checkError}</p>
					<p class="text-secondary">
						Fix it and try again, or keep it as is. Ask the AI what the error means.
					</p>
				</div>
			{/if}

			{#if testResult && !saveError && !checkError}
				<p
					class="break-words text-2xs {testResult.ok
						? 'text-green-600 dark:text-green-400'
						: 'text-red-600 dark:text-red-400'}"
				>
					{testResult.ok ? 'The test passed.' : `The test failed: ${testResult.error}`}
				</p>
			{/if}

			<div class="flex justify-end gap-2">
				{#if testable}
					<Button
						variant="subtle"
						unifiedSize="sm"
						wrapperClasses="mr-auto"
						startIcon={{ icon: PlugZap }}
						disabled={!interactive || saving || !allSecrets}
						loading={testing}
						onClick={() => void testNow()}
					>
						Test
					</Button>
				{/if}
				<Button
					variant="default"
					unifiedSize="sm"
					disabled={!interactive || saving}
					onClick={() => onDismiss?.()}
				>
					{checkError ? 'Keep anyway' : 'Dismiss'}
				</Button>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!canSave}
					onClick={() => void submit()}
					loading={saving}
					startIcon={{ icon: Lock }}
				>
					{#if checkError}
						Try again
					{:else if replaceDrafts}
						Replace drafts
					{:else if tested}
						{taken ? 'Update draft and test' : 'Save draft and test'}
					{:else}
						{taken ? 'Update draft' : 'Save draft'}
					{/if}
				</Button>
			</div>
		</div>
	</div>
{/if}
