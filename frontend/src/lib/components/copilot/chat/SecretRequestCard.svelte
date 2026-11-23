<script lang="ts">
	import { ExternalLink, KeyRound, Lock, ShieldCheck } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Password from '$lib/components/Password.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import Path from '$lib/components/Path.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { safeHref } from '$lib/utils/safeHref'
	import {
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

	// The model writes this URL, so a prompt injection could make it `javascript:`.
	const helpHref = $derived(safeHref(request.helpUrl, window.location.href))
	const resource = $derived(request.resource)
	const secretFields = $derived(secretFieldsOf(request))

	let secrets = $state<Record<string, string>>({})
	// Only what the user changed; the rest stays as the AI prepared it.
	let fieldEdits = $state<Record<string, string>>({})
	let submitting = $state(false)
	const saving = $derived(submitting || !!request.saving)
	let saveError = $state<string | undefined>()

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
	const variablePaths = $derived(secretVariablePaths(savePath, secretFields))
	const canSave = $derived(
		interactive &&
			secretFields.every((f) => (secrets[f.field] ?? '').length > 0) &&
			!saving &&
			(!pathError || taken)
	)

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
		try {
			await save({
				path: savePath,
				// The user asked the AI to replace what is there, which the path picker may not show:
				// it checks the resource path, not the variables beside it.
				replace: taken || !!request.replace,
				secrets: $state.snapshot(secrets),
				fields: submittedFields()
			})
			secrets = {}
		} catch (e) {
			saveError = e instanceof Error ? e.message : String(e)
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

{#if request.saved}
	<!-- Settled, it reads as one more tool row in the turn, not a card. -->
	<div class="my-0.5 flex min-w-0 items-start gap-2 py-0.5 font-mono text-2xs leading-4">
		<ShieldCheck class="mt-px h-3.5 w-3.5 shrink-0 text-green-600 dark:text-green-400" />
		<span class="min-w-0 font-medium text-secondary">
			{#if resource}
				{request.saved.replaced ? 'Updated' : 'Connected'}
				{request.label} as
				<a
					class="text-accent hover:underline"
					href={resourcePageUrl(request.saved.path, request.workspace)}>{request.saved.path}</a
				>
				· secrets in
				{#each request.saved.variables as variable, i (variable)}
					{#if i > 0},
					{/if}
					<a class="text-accent hover:underline" href={variablePageUrl(variable, request.workspace)}
						>{variable}</a
					>
				{/each}
			{:else}
				{request.saved.replaced ? 'Replaced' : 'Saved'}
				{request.label} in secret variable
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
			<span class="text-2xs text-secondary">· saved by Windmill, not sent to the AI</span>
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
				</div>
				{#if helpHref}
					<a
						class="flex shrink-0 items-center gap-1 text-2xs text-accent"
						href={helpHref}
						target="_blank"
						rel="noopener noreferrer"
					>
						Where to find it <ExternalLink class="h-3 w-3" />
					</a>
				{/if}
			</div>

			{#each secretFields as secretField (secretField.field)}
				<div class="flex flex-col gap-1">
					{#if resource}
						<span class="text-2xs font-medium text-secondary">{secretField.label}</span>
					{/if}
					<Password
						bind:password={secrets[secretField.field]}
						placeholder="Paste the value here"
						disabled={!interactive || saving}
						autocomplete="off"
						error={saveError !== undefined}
						onKeyDown={onSecretKeyDown}
					/>
				</div>
			{/each}

			{#if resource}
				{#each Object.entries(resource.value) as [key, fieldValue] (key)}
					<div class="flex flex-col gap-1">
						<span class="text-2xs font-medium text-secondary">{key}</span>
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
				<span class="text-2xs font-medium text-secondary">
					{resource ? 'Saved as the resource' : 'Saved as the secret variable'}
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
						{#if resource}
							Encrypted and stored in this workspace as secret
							{variablePaths.length === 1 ? 'variable' : 'variables'}
							<span class="font-mono">{variablePaths.join(', ')}</span>.
						{:else}
							Encrypted and stored in this workspace.
						{/if}
					</span>
				</li>
				<li class="flex items-start gap-1.5">
					<ShieldCheck class="mt-0.5 h-3 w-3 shrink-0 text-green-600 dark:text-green-400" />
					<span>The AI never receives the values, only the path to reference them.</span>
				</li>
			</ul>

			{#if saveError}
				<p class="text-2xs text-red-600 dark:text-red-400">Could not save: {saveError}</p>
			{/if}

			<div class="flex justify-end gap-2">
				<Button
					variant="default"
					unifiedSize="sm"
					disabled={!interactive || saving}
					onClick={() => onDismiss?.()}
				>
					Dismiss
				</Button>
				<Button
					variant="accent"
					unifiedSize="sm"
					disabled={!canSave}
					onClick={() => void submit()}
					loading={saving}
					startIcon={{ icon: Lock }}
				>
					{#if resource}
						{taken ? 'Update resource' : 'Save and connect'}
					{:else}
						{taken ? 'Update secret' : 'Save secret'}
					{/if}
				</Button>
			</div>
		</div>
	</div>
{/if}
