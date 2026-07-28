<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { ScriptService, type Preview, type Script } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { Loader2, Save, DiffIcon, Settings } from 'lucide-svelte'
	import ScriptAdvancedSettings from '$lib/components/ScriptAdvancedSettings.svelte'
	import ScriptSettingsBadges from '$lib/components/ScriptSettingsBadges.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import {
		cleanValueProperties,
		emptySchema,
		emptyString,
		orderedJsonStringify,
		sendUserToast
	} from '$lib/utils'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import WorkerTagSelect from '$lib/components/WorkerTagSelect.svelte'
	import type { FlowEditorContext } from '../types'

	let scriptEditorDrawer: Drawer | undefined = $state()

	const dispatch = createEventDispatcher()

	const flowEditorContext = getContext<FlowEditorContext>('FlowEditorContext')
	let opWs = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	export async function openDrawer(hash: string, cb: () => void): Promise<void> {
		script = undefined
		closeAnyway = false
		scriptEditorDrawer?.openDrawer?.()
		script = await ScriptService.getScriptByHash({
			workspace: opWs!,
			hash
		})
		savedScript = structuredClone($state.snapshot(script))
		callback = cb
	}

	let callback: (() => void) | undefined = undefined
	let script:
		| {
				path: string
				description: string
				summary: string
				hash: string
				language: Preview['language']
				content: string
				schema?: any
				tag?: string
				kind: 'script' | 'failure' | 'trigger' | 'command' | 'approval' | 'preprocessor' | undefined
				envs?: string[]
				ws_error_handler_muted?: boolean
				dedicated_worker?: boolean
				visible_to_runner_only?: boolean
				on_behalf_of_email?: string
				auto_kind?: string
				has_preprocessor?: boolean
				concurrent_limit?: number
				concurrency_time_window_s?: number
				concurrency_key?: string
				cache_ttl?: number
				cache_ignore_s3_path?: boolean
				timeout?: number
				debounce_delay_s?: number
				debounce_key?: string
				debounce_args_to_accumulate?: string[]
				max_total_debouncing_time?: number
				max_total_debounces_amount?: number
				restart_unless_cancelled?: boolean
				priority?: number
				delete_after_secs?: number
		  }
		| undefined = $state(undefined)

	let savedScript:
		| {
				path: string
				description: string
				summary: string
				hash: string
				language: Preview['language']
				content: string
				schema?: any
				tag?: string
				kind: 'script' | 'failure' | 'trigger' | 'command' | 'approval' | 'preprocessor' | undefined
				envs?: string[]
				ws_error_handler_muted?: boolean
				dedicated_worker?: boolean
				visible_to_runner_only?: boolean
				on_behalf_of_email?: string
				auto_kind?: string
				has_preprocessor?: boolean
				concurrent_limit?: number
				concurrency_time_window_s?: number
				concurrency_key?: string
				cache_ttl?: number
				cache_ignore_s3_path?: boolean
				timeout?: number
				debounce_delay_s?: number
				debounce_key?: string
				debounce_args_to_accumulate?: string[]
				max_total_debouncing_time?: number
				max_total_debounces_amount?: number
				restart_unless_cancelled?: boolean
				priority?: number
				delete_after_secs?: number
		  }
		| undefined = $state(undefined)

	async function saveScript(): Promise<void> {
		if (script) {
			try {
				script.schema = script.schema ?? emptySchema()
				try {
					const result = await inferArgs(script.language, script.content, script.schema)
					script.auto_kind = result?.auto_kind || undefined
					script.has_preprocessor = result?.has_preprocessor || undefined
				} catch (error) {
					sendUserToast(`Could not parse code, are you sure it is valid?`, true)
				}

				await ScriptService.createScript({
					workspace: opWs!,
					requestBody: {
						...script,
						language: script.language!,
						description: script.description ?? '',
						parent_hash: script.hash != '' ? script.hash : undefined,
						is_template: false,
						tag: script.tag,
						kind: script.kind as Script['kind'] | undefined,
						// Empty keys are shared global keys server-side; treat cleared inputs as unset.
						concurrency_key: emptyString(script.concurrency_key)
							? undefined
							: script.concurrency_key,
						debounce_key: emptyString(script.debounce_key) ? undefined : script.debounce_key,
						lock: undefined
					}
				})
				savedScript = structuredClone($state.snapshot(script))
				callback?.()
			} catch (error) {
				sendUserToast(`Impossible to save the script: ${error.body}`, true)
			}
		}
	}

	let closeAnyway = $state(false)
	let diffDrawer: DiffDrawer | undefined = $state()
	let unsavedModalOpen = $state(false)
	async function checkForUnsavedChanges() {
		if (closeAnyway) {
			scriptEditorDrawer?.closeDrawer()
			return
		}
		if (savedScript && script) {
			const saved = cleanValueProperties(savedScript)
			const current = cleanValueProperties(script)
			if (orderedJsonStringify(saved) !== orderedJsonStringify(current)) {
				unsavedModalOpen = true
				scriptEditorDrawer?.openDrawer()
			} else {
				scriptEditorDrawer?.closeDrawer()
			}
		}
	}
	let args = $state({})

	let displayEditor = $state(true)
	let settingsDrawer: Drawer | undefined = $state()
</script>

<ConfirmationModal
	open={unsavedModalOpen}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	on:canceled={() => {
		unsavedModalOpen = false
	}}
	on:confirmed={() => {
		console.log('confirmed')
		closeAnyway = true
		unsavedModalOpen = false
		scriptEditorDrawer?.closeDrawer()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to discard the changes you have made? </span>
		<Button
			wrapperClasses="self-start"
			variant="default"
			size="xs"
			on:click={() => {
				if (!savedScript || !script) {
					return
				}
				unsavedModalOpen = false
				closeAnyway = true
				displayEditor = false
				diffDrawer?.openDrawer()
				diffDrawer?.setDiff({
					title: 'Saved <> Current',
					mode: 'simple',
					original: savedScript,
					current: script,
					button: {
						text: 'Close anyway',
						onClick: () => {
							closeAnyway = true
							diffDrawer?.closeDrawer()
						}
					}
				})
			}}
			>Show diff
		</Button>
	</div>
</ConfirmationModal>
<!-- <div id="monaco-widgets-root" class="monaco-editor" style="z-index: 1200;" /> -->

<Drawer
	bind:this={scriptEditorDrawer}
	size="1200px"
	on:close={() => {
		checkForUnsavedChanges()
	}}
>
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
		fullScreen
		on:close={() => {
			scriptEditorDrawer?.closeDrawer()
		}}
	>
		{#if script && displayEditor}
			{#key script.hash}
				<ScriptEditor
					workspaceOverride={opWs}
					showCaptures={false}
					on:saveDraft={() => {
						saveScript()
					}}
					noSyncFromGithub
					lang={script.language}
					path={script.path}
					autoKind={script.auto_kind}
					tag={script.tag}
					fixedOverflowWidgets={false}
					bind:code={script.content}
					bind:schema={script.schema}
					{args}
				>
					{#snippet editorBarRight()}
						<div>
							<WorkerTagSelect
								bind:tag={() => script?.tag, (v) => script && (script.tag = v)}
								workspaceId={opWs}
							/>
						</div>
					{/snippet}
				</ScriptEditor>
			{/key}
		{:else}
			<div
				out:fade={{ duration: 200 }}
				class="absolute inset-0 center-center flex-col bg-surface text-primary border"
			>
				<Loader2 class="animate-spin" size={16} />
				<span class="text-xs mt-1">Loading</span>
			</div>
		{/if}
		{#snippet actions()}
			{#if script}
				<ScriptSettingsBadges settings={script} onclick={() => settingsDrawer?.openDrawer()} />
			{/if}
			<Popover notClickable placement="bottom">
				<Button
					disabled={!script}
					variant="default"
					iconOnly
					startIcon={{ icon: Settings }}
					aria-label="Runtime settings"
					on:click={() => settingsDrawer?.openDrawer()}
				/>
				{#snippet text()}Runtime settings (concurrency, cache, timeout, ...){/snippet}
			</Popover>
			<Button
				disabled={!savedScript || !script}
				variant="default"
				on:click={async () => {
					if (!savedScript || !script) {
						return
					}
					closeAnyway = true
					displayEditor = false
					diffDrawer?.openDrawer()
					diffDrawer?.setDiff({
						mode: 'simple',
						original: savedScript,
						current: script,
						title: 'Saved <> Current',
						button: {
							text: 'Restore to saved',
							onClick: () => {
								script = structuredClone($state.snapshot(savedScript))
								diffDrawer?.closeDrawer()
							}
						}
					})
				}}
			>
				<div class="flex flex-row gap-2 items-center">
					<DiffIcon size={14} />
					Diff
				</div>
			</Button>
			<Button
				on:click={async () => {
					await saveScript()
					dispatch('save')
					scriptEditorDrawer?.closeDrawer()
				}}
				disabled={!script}
				startIcon={{ icon: Save }}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>

<DiffDrawer
	bind:this={diffDrawer}
	on:close={() => {
		displayEditor = true
	}}
/>

<Drawer bind:this={settingsDrawer} size="600px">
	<DrawerContent title="Script settings" on:close={() => settingsDrawer?.closeDrawer()}>
		{#if script}
			<div class="flex flex-col gap-4">
				<p class="text-xs text-secondary">
					These runtime settings are saved together with the script when you press Save.
				</p>
				<ScriptAdvancedSettings {script} workspaceId={opWs} />
			</div>
		{/if}
		{#snippet actions()}
			<Button variant="border" on:click={() => settingsDrawer?.closeDrawer()}>Done</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
