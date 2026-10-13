<script lang="ts">
	import { Alert, Skeleton } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import SettingsFooter from './SettingsFooter.svelte'
	import { WorkspaceService } from '$lib/gen'
	import type { ApplyWorkspaceSettingsDocumentResponse } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2 } from 'lucide-svelte'
	import YAML from 'yaml'
	import { untrack } from 'svelte'
	import type SimpleEditor from '$lib/components/SimpleEditor.svelte'

	let {
		onApplied
	}: {
		/** Called after the document was applied, so the form panes can reload what they show. */
		onApplied?: () => void | Promise<void>
	} = $props()

	type Change = ApplyWorkspaceSettingsDocumentResponse['changes'][number]

	// Keys the server never applies from the document. `color` and `operator_settings` are applied
	// when set but a null one is ignored, so deleting them does not clear them either.
	const READ_ONLY_KEYS = ['name', 'slack_team_id', 'slack_name']
	const NOT_CLEARABLE_KEYS = ['color', 'operator_settings']

	let loading = $state(true)
	let code = $state('')
	let initialCode = $state('')
	let loadedKeys: string[] = []
	let editor: SimpleEditor | undefined = $state(undefined)

	let parseError = $state('')
	let applyError = $state('')
	let warnings: string[] = $state([])

	let pendingDocument: Record<string, unknown> | undefined = undefined
	let pendingChanges: Change[] = $state([])
	let confirmOpen = $state(false)
	let applying = $state(false)

	async function load() {
		loading = true
		try {
			const doc = await WorkspaceService.getWorkspaceSettingsDocument({
				workspace: $workspaceStore!
			})
			loadedKeys = Object.keys(doc)
			code = YAML.stringify(doc)
			initialCode = code
			editor?.setCode(code)
			parseError = ''
		} catch (e) {
			applyError = errorMessage(e)
		} finally {
			loading = false
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => load())
		}
	})

	function errorMessage(e: unknown): string {
		const body = (e as { body?: unknown })?.body
		if (typeof body === 'string' && body) return body
		return e instanceof Error ? e.message : String(e)
	}

	/** The document to send: a key the editor no longer has becomes `null`, which clears it. */
	function documentFromEditor(): Record<string, unknown> | undefined {
		const current = editor?.getCode() ?? code
		code = current
		let parsed: unknown
		try {
			parsed = YAML.parse(current)
		} catch (e) {
			parseError = String(e)
			return undefined
		}
		if (parsed == null) parsed = {}
		if (typeof parsed !== 'object' || Array.isArray(parsed)) {
			parseError = 'The document must be a mapping (key: value)'
			return undefined
		}
		parseError = ''
		const doc = { ...(parsed as Record<string, unknown>) }
		for (const key of loadedKeys) {
			if (!(key in doc)) doc[key] = null
		}
		return doc
	}

	async function review(): Promise<boolean> {
		applyError = ''
		warnings = []
		const document = documentFromEditor()
		if (!document) return false
		try {
			const res = await WorkspaceService.applyWorkspaceSettingsDocument({
				workspace: $workspaceStore!,
				requestBody: { document, replace: false, dry_run: true }
			})
			warnings = res.warnings ?? []
			if (res.changes.length === 0) {
				sendUserToast('No setting would change')
				return false
			}
			pendingDocument = document
			pendingChanges = res.changes
			confirmOpen = true
		} catch (e) {
			applyError = errorMessage(e)
		}
		return false
	}

	async function apply() {
		if (!pendingDocument) return
		applying = true
		let ok = false
		try {
			const res = await WorkspaceService.applyWorkspaceSettingsDocument({
				workspace: $workspaceStore!,
				requestBody: { document: pendingDocument, replace: false }
			})
			warnings = res.warnings ?? []
			ok = true
			sendUserToast(`Applied ${res.changes.length} setting change(s)`)
		} catch (e) {
			applyError = errorMessage(e)
		} finally {
			applying = false
			confirmOpen = false
			pendingDocument = undefined
		}
		// A failing write stops the apply but keeps the writes before it, so the form panes reload
		// either way; the editor keeps the user's text on failure so it can be fixed and retried.
		await Promise.all([ok ? load() : undefined, onApplied?.()])
	}

	function discard() {
		code = initialCode
		editor?.setCode(initialCode)
		parseError = ''
		applyError = ''
		warnings = []
	}

	export function unsavedChanges() {
		return {
			savedValue: { code: initialCode },
			modifiedValue: { code: editor?.getCode() ?? code }
		}
	}

	function formatValue(v: unknown): string {
		if (v === null || v === undefined) return 'unset'
		const s = typeof v === 'string' ? v : JSON.stringify(v)
		return s.length > 200 ? s.slice(0, 200) + '…' : s
	}
</script>

<div class="flex flex-col gap-2 h-full">
	<p class="text-xs text-secondary">
		The workspace settings in the layout of the <code>settings.yaml</code> file that
		<code>wmill sync</code> reads and writes. Deleting a key clears that setting. Secrets show as
		<code>__SENSITIVE_AND_UNCHANGED__</code>: leave the placeholder to keep the stored value.
	</p>
	<Alert type="info" title="Read-only keys" size="xs">
		{#each READ_ONLY_KEYS as key, i (key)}<code>{key}</code>{i < READ_ONLY_KEYS.length - 1
				? ', '
				: ''}{/each} are shown for reference and never applied: change the name from the General pane
		and connect Slack from the Slack pane.
		{#each NOT_CLEARABLE_KEYS as key, i (key)}<code>{key}</code>{i < NOT_CLEARABLE_KEYS.length - 1
				? ' and '
				: ''}{/each} are applied when set, but deleting them does not clear them.
	</Alert>

	<div class="border rounded w-full h-[calc(100vh-22rem)] min-h-64">
		{#if loading && !initialCode}
			<Skeleton layout={[[20]]} />
		{:else}
			{#await import('$lib/components/SimpleEditor.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default
					bind:this={editor}
					class="h-full"
					lang="yaml"
					bind:code
					fixedOverflowWidgets={false}
				/>
			{/await}
		{/if}
	</div>

	{#if parseError}
		<Alert type="error" title="Invalid YAML" size="xs">{parseError}</Alert>
	{/if}
	{#if applyError}
		<Alert type="error" title="The settings could not be applied" size="xs">{applyError}</Alert>
	{/if}
	{#each warnings as warning, i (i)}
		<Alert type="warning" title="Warning" size="xs">{warning}</Alert>
	{/each}

	<SettingsFooter
		inline
		hasUnsavedChanges={code !== initialCode}
		saveLabel="Review and save"
		onSave={review}
		onDiscard={discard}
	/>
</div>

<ConfirmationModal
	open={confirmOpen}
	title="Apply these changes?"
	confirmationText="Apply"
	type="info"
	loading={applying}
	onConfirmed={apply}
	onCanceled={() => {
		confirmOpen = false
		pendingDocument = undefined
	}}
>
	<div class="flex flex-col gap-2 max-h-96 overflow-auto">
		{#each pendingChanges as change (change.key)}
			<div class="flex flex-col gap-0.5">
				<span class="text-xs font-semibold text-emphasis font-mono">{change.key}</span>
				<span class="text-2xs text-secondary font-mono break-all">
					{formatValue(change.before)} → {formatValue(change.after)}
				</span>
			</div>
		{/each}
	</div>
</ConfirmationModal>
