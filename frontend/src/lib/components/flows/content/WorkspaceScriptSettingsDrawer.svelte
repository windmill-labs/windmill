<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ScriptAdvancedSettings from '$lib/components/ScriptAdvancedSettings.svelte'
	import ScriptSettingsBadges from '$lib/components/ScriptSettingsBadges.svelte'
	import { ScriptService, type Script } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/utils'
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

	// Open the settings drawer for the workspace script referenced by `path`.
	// A specific `hash` may be passed to load that version, but saving always
	// creates a new version whose parent is resolved to the current deployed head.
	export async function openDrawer(
		path: string,
		hash: string | undefined,
		cb?: () => void
	): Promise<void> {
		script = undefined
		callback = cb
		drawer?.openDrawer?.()
		loading = true
		try {
			script = hash
				? await ScriptService.getScriptByHash({ workspace: opWs!, hash })
				: await ScriptService.getScriptByPath({ workspace: opWs!, path })
		} catch (e) {
			sendUserToast(`Could not load script settings: ${e}`, true)
		} finally {
			loading = false
		}
	}

	async function save(): Promise<void> {
		if (!script) return
		saving = true
		try {
			await ScriptService.createScript({
				workspace: opWs!,
				requestBody: {
					path: script.path,
					summary: script.summary ?? '',
					description: script.description ?? '',
					content: script.content,
					schema: script.schema,
					language: script.language,
					kind: script.kind,
					parent_hash: script.hash,
					auto_parent: true,
					is_template: false,
					lock: script.lock,
					tag: script.tag,
					envs: script.envs,
					concurrent_limit: script.concurrent_limit,
					concurrency_time_window_s: script.concurrency_time_window_s,
					concurrency_key: script.concurrency_key,
					cache_ttl: script.cache_ttl,
					cache_ignore_s3_path: script.cache_ignore_s3_path,
					dedicated_worker: script.dedicated_worker,
					priority: script.priority,
					restart_unless_cancelled: script.restart_unless_cancelled,
					timeout: script.timeout,
					delete_after_secs: script.delete_after_secs,
					debounce_key: script.debounce_key,
					debounce_delay_s: script.debounce_delay_s,
					debounce_args_to_accumulate: script.debounce_args_to_accumulate,
					max_total_debouncing_time: script.max_total_debouncing_time,
					max_total_debounces_amount: script.max_total_debounces_amount,
					visible_to_runner_only: script.visible_to_runner_only,
					ws_error_handler_muted: script.ws_error_handler_muted,
					on_behalf_of_email: script.on_behalf_of_email,
					has_preprocessor: script.has_preprocessor,
					auto_kind: script.auto_kind
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
