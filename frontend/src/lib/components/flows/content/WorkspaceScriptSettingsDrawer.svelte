<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ScriptAdvancedSettings from '$lib/components/ScriptAdvancedSettings.svelte'
	import ScriptSettingsBadges from '$lib/components/ScriptSettingsBadges.svelte'
	import { ScriptService, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString, sendUserToast } from '$lib/utils'
	import { Loader2, Save } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	let opWs = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	let drawer: Drawer | undefined = $state()
	// cache_ignore_s3_path lives on NewScript but not the returned Script type;
	// ScriptAdvancedSettings still edits it, so widen the local type to keep it.
	let script: (Script & { cache_ignore_s3_path?: boolean }) | undefined = $state(undefined)
	let loading = $state(false)
	let saving = $state(false)
	let callback: (() => void) | undefined = undefined
	// Guards against a slow load for a previously-opened script resolving after
	// the drawer was reopened for a different one and overwriting its target.
	let openSeq = 0

	// Open the settings drawer for the workspace script referenced by `path`.
	// A specific `hash` may be passed to load that version.
	export async function openDrawer(
		path: string,
		hash: string | undefined,
		cb?: () => void
	): Promise<void> {
		const seq = ++openSeq
		script = undefined
		callback = cb
		drawer?.openDrawer?.()
		loading = true
		try {
			const loaded = hash
				? await ScriptService.getScriptByHash({ workspace: opWs!, hash })
				: await ScriptService.getScriptByPath({ workspace: opWs!, path })
			if (seq !== openSeq) return
			script = loaded
		} catch (e) {
			if (seq === openSeq) sendUserToast(`Could not load script settings: ${e}`, true)
		} finally {
			if (seq === openSeq) loading = false
		}
	}

	async function save(): Promise<void> {
		if (!script) return
		saving = true
		try {
			// Spread the full loaded script so fields we don't edit here (codebase,
			// labels, envs, on_behalf_of_email, ...) survive the new version; only
			// override lineage and normalize the edited settings.
			// parent_hash without auto_parent is an optimistic-concurrency guard: if the
			// script was deployed elsewhere while the drawer was open, the save fails
			// loudly (non-linear lineage) instead of silently reverting that deploy.
			// preserve_on_behalf_of/skip_draft_deletion keep a settings-only save from
			// hijacking the execution identity or discarding the author's code draft.
			await ScriptService.createScript({
				workspace: opWs!,
				requestBody: {
					...script,
					summary: script.summary ?? '',
					description: script.description ?? '',
					parent_hash: script.hash,
					is_template: false,
					lock: script.lock,
					concurrency_key: emptyString(script.concurrency_key) ? undefined : script.concurrency_key,
					debounce_key: emptyString(script.debounce_key) ? undefined : script.debounce_key,
					preserve_on_behalf_of: true,
					skip_draft_deletion: true
				}
			})
			sendUserToast('Script settings saved')
			callback?.()
			drawer?.closeDrawer()
		} catch (e) {
			sendUserToast(`Could not save script settings: ${e.body ?? e}`, true)
		} finally {
			saving = false
		}
	}
</script>

<Drawer bind:this={drawer} size="600px">
	<DrawerContent title="Script settings" on:close={() => drawer?.closeDrawer()}>
		{#if loading || !script}
			<div class="center-center flex-col h-full text-tertiary">
				<Loader2 class="animate-spin" size={16} />
				<span class="text-xs mt-1">Loading</span>
			</div>
		{:else}
			<div class="flex flex-col gap-4">
				<div class="flex flex-row items-center gap-2 flex-wrap">
					<span class="text-xs text-tertiary">{script.path}</span>
					<ScriptSettingsBadges settings={script} />
				</div>
				<p class="text-xs text-secondary">
					Saving creates a new version of the workspace script with these runtime settings. The code
					is left unchanged.
				</p>
				<ScriptAdvancedSettings {script} workspaceId={opWs} />
			</div>
		{/if}
		{#snippet actions()}
			<Button
				on:click={save}
				disabled={!script || loading || saving}
				startIcon={{ icon: saving ? Loader2 : Save }}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
