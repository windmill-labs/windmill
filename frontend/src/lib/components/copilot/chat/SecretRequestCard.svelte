<script lang="ts">
	import { ExternalLink, KeyRound, Lock, ShieldCheck } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Password from '$lib/components/Password.svelte'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import Path from '$lib/components/Path.svelte'
	import { safeHref } from '$lib/utils/safeHref'
	import { resourcePageUrl, saveSecretVariable, variablePageUrl } from './secretRequest'
	import type { SecretRequestDisplay, SecretRequestOutcome } from './shared'

	interface Props {
		request: SecretRequestDisplay
		/** Inert copies (an answered request in history) render the form but cannot save. */
		interactive?: boolean
		onSaved?: (outcome: SecretRequestOutcome) => void
		onDismiss?: () => void
	}

	let {
		request,
		interactive = true,
		onSaved,
		onDismiss
	}: Props = $props()

	// The model writes this URL, so a prompt injection could make it `javascript:`.
	const helpHref = $derived(safeHref(request.helpUrl, window.location.href))

	let value = $state('')
	let saving = $state(false)
	let saveError = $state<string | undefined>()

	// A taken path is the only error that still lets the user save: saving then updates the
	// variable that is there.
	const TAKEN = 'This path is already taken. Pick another or update the secret already there.'
	let pickedPath = $state<string | undefined>()
	let pathError = $state<string | undefined>()
	const savePath = $derived(pickedPath || request.path)
	const taken = $derived(pathError === TAKEN)
	const canSave = $derived(interactive && value.length > 0 && !saving && (!pathError || taken))

	async function submit() {
		if (!canSave) return
		saving = true
		saveError = undefined
		try {
			const outcome = await saveSecretVariable(request, value, savePath, taken)
			value = ''
			onSaved?.(outcome)
		} catch (e) {
			saveError = e instanceof Error ? e.message : String(e)
		} finally {
			saving = false
		}
	}
</script>

{#if request.saved}
	<!-- Settled, it reads as one more tool row in the turn, not a card. -->
	<div class="my-0.5 flex min-w-0 items-start gap-2 py-0.5 font-mono text-2xs leading-4">
		<ShieldCheck class="mt-px h-3.5 w-3.5 shrink-0 text-green-600 dark:text-green-400" />
		<span class="min-w-0 font-medium text-secondary">
			{request.saved.replaced ? 'Replaced' : 'Saved'}
			{request.label} in secret variable
			<a class="text-accent hover:underline" href={variablePageUrl(request.saved.path, request.workspace)}
				>{request.saved.path}</a
			>
			{#if request.resourcePath}
				· used by
				<a class="text-accent hover:underline" href={resourcePageUrl(request.resourcePath, request.workspace)}
					>{request.resourcePath}</a
				>
			{/if}
		</span>
	</div>
{:else if request.canceled}
	<div class="my-0.5 flex min-w-0 items-start gap-2 py-0.5 font-mono text-2xs leading-4">
		<Lock class="mt-px h-3.5 w-3.5 shrink-0 text-tertiary" />
		<span class="min-w-0 font-medium text-tertiary"
			>Secret request for {request.label} dismissed</span
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
				{#if request.resourceType}
					<div class="shrink-0 pt-0.5">
						<IconedResourceType name={request.resourceType} silent width="20px" height="20px" />
					</div>
				{:else}
					<KeyRound class="mt-0.5 h-4 w-4 shrink-0 text-secondary" />
				{/if}
				<div class="min-w-0 flex-1">
					<p class="text-xs font-semibold text-emphasis">{request.label}</p>
					<p class="text-2xs text-secondary">
						The AI needs this to
						{#if request.resourcePath}
							connect <span class="font-mono">{request.resourcePath}</span>.
						{:else}
							continue.
						{/if}
						{#if request.description}
							{request.description}
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

			<Password
				bind:password={value}
				placeholder="Paste the value here"
				disabled={!interactive || saving}
				autocomplete="off"
				error={saveError !== undefined}
				onKeyDown={(e) => {
					// Enter is a newline: a pasted private key keeps its line breaks.
					if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
						e.preventDefault()
						void submit()
					}
				}}
			/>

			<div class="flex flex-col gap-1">
				<span class="text-2xs font-medium text-secondary">Saved as the secret variable</span>
				<Path
					kind="variable"
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
						Encrypted and stored in this workspace.
					</span>
				</li>
				<li class="flex items-start gap-1.5">
					<ShieldCheck class="mt-0.5 h-3 w-3 shrink-0 text-green-600 dark:text-green-400" />
					<span>
						The AI never receives the value, only the path to reference it.
					</span>
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
					{taken ? 'Update secret' : 'Save secret'}
				</Button>
			</div>
		</div>
	</div>
{/if}
