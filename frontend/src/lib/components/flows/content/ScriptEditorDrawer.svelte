<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { ScriptService, type Preview, Script } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { Loader2, Save, DiffIcon } from 'lucide-svelte'
	import {
		cleanValueProperties,
		emptySchema,
		orderedJsonStringify,
		sendUserToast
	} from '$lib/utils'
	import { cloneDeep } from 'lodash'
	import { createEventDispatcher } from 'svelte'
	import { fade } from 'svelte/transition'

	let scriptEditorDrawer: Drawer

	const dispatch = createEventDispatcher()

	export async function openDrawer(hash: string, cb: () => void): Promise<void> {
		script = undefined
		scriptEditorDrawer.openDrawer?.()
		script = await ScriptService.getScriptByHash({
			workspace: $workspaceStore!,
			hash
		})
		savedScript = cloneDeep(script)
		callback = cb
	}

	let callback: (() => void) | undefined = undefined
	let script:
		| {
				path: string
				description: string
				summary: string
				hash: string
				language: Preview.language
				content: string
				schema?: any
				kind: 'script' | 'failure' | 'trigger' | 'command' | 'approval' | undefined
				envs?: string[]
				ws_error_handler_muted?: boolean
		  }
		| undefined = undefined

	let savedScript:
		| {
				path: string
				description: string
				summary: string
				hash: string
				language: Preview.language
				content: string
				schema?: any
				kind: 'script' | 'failure' | 'trigger' | 'command' | 'approval' | undefined
				envs?: string[]
				ws_error_handler_muted?: boolean
		  }
		| undefined = undefined

	async function saveScript(): Promise<void> {
		if (script) {
			try {
				script.schema = script.schema ?? emptySchema()
				try {
					await inferArgs(script.language, script.content, script.schema)
				} catch (error) {
					sendUserToast(
						`Impossible to infer the schema. Assuming this is a script without main function`,
						true
					)
				}

				await ScriptService.createScript({
					workspace: $workspaceStore!,
					requestBody: {
						path: script.path,
						summary: script.summary,
						description: script.description ?? '',
						content: script.content,
						parent_hash: script.hash != '' ? script.hash : undefined,
						schema: script.schema,
						is_template: false,
						language: script.language,
						kind: script.kind as Script.kind | undefined,
						envs: script.envs,
						ws_error_handler_muted: script.ws_error_handler_muted
					}
				})
				savedScript = cloneDeep(script)
				callback?.()
			} catch (error) {
				sendUserToast(`Impossible to save the script: ${error.body}`, true)
			}
		}
	}

	let closeAnyway = false
	let diffDrawer: DiffDrawer
	let unsavedModalOpen = false
	async function checkForUnsavedChanges() {
		if (closeAnyway) {
			scriptEditorDrawer.closeDrawer()
			closeAnyway = false
			return
		}
		if (savedScript && script) {
			const saved = cleanValueProperties(savedScript)
			const current = cleanValueProperties(script)
			if (orderedJsonStringify(saved) !== orderedJsonStringify(current)) {
				unsavedModalOpen = true
			} else {
				scriptEditorDrawer.closeDrawer()
			}
		}
	}
</script>

<!-- <UnsavedConfirmationModal savedValue={savedScript} modifiedValue={script} {diffDrawer} /> -->

<ConfirmationModal
	open={unsavedModalOpen}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	on:canceled={() => {
		unsavedModalOpen = false
	}}
	on:confirmed={() => {
		unsavedModalOpen = false
		closeAnyway = true
		scriptEditorDrawer.closeDrawer()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to discard the changes you have made? </span>
		<Button
			wrapperClasses="self-start"
			color="light"
			variant="border"
			size="xs"
			on:click={() => {
				if (!savedScript || !script) {
					return
				}
				unsavedModalOpen = false
				closeAnyway = true
				scriptEditorDrawer.closeDrawer()
				diffDrawer.openDrawer()
				diffDrawer.setDiff({
					title: 'Saved <> Current',
					mode: 'simple',
					original: savedScript,
					current: script,
					button: {
						text: 'Close anyway',
						onClick: () => {
							closeAnyway = true
							diffDrawer.closeDrawer()
						}
					}
				})
			}}
			>Show diff
		</Button>
	</div>
</ConfirmationModal>
<Drawer
	bind:this={scriptEditorDrawer}
	size="1200px"
	on:close={() => {
		scriptEditorDrawer.openDrawer()
		checkForUnsavedChanges()
	}}
>
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
		fullScreen
		on:close={() => {
			scriptEditorDrawer.closeDrawer()
		}}
	>
		{#if script}
			{#key script.hash}
				<ScriptEditor
					noSyncFromGithub
					lang={script.language}
					path={script.path}
					fixedOverflowWidgets={true}
					bind:code={script.content}
					bind:schema={script.schema}
					tag={undefined}
				/>
			{/key}
		{:else}
			<div
				out:fade={{ duration: 200 }}
				class="absolute inset-0 center-center flex-col bg-surface text-tertiary border"
			>
				<Loader2 class="animate-spin" size={16} />
				<span class="text-xs mt-1">Loading</span>
			</div>
		{/if}
		<svelte:fragment slot="actions">
			<Button
				disabled={!savedScript || !script}
				color="light"
				variant="border"
				on:click={async () => {
					if (!savedScript || !script) {
						return
					}
					closeAnyway = true
					scriptEditorDrawer.closeDrawer()
					diffDrawer.openDrawer()
					diffDrawer.setDiff({
						mode: 'simple',
						original: savedScript,
						current: script,
						title: 'Saved <> Current',
						button: {
							text: 'Restore to saved',
							onClick: () => {
								script = cloneDeep(savedScript)
								diffDrawer.closeDrawer()
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
					scriptEditorDrawer.closeDrawer()
				}}
				disabled={!script}
				startIcon={{ icon: Save }}
			>
				Save
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>

<DiffDrawer
	bind:this={diffDrawer}
	on:close={() => {
		if (!closeAnyway) {
			scriptEditorDrawer.openDrawer()
		} else {
			closeAnyway = false
		}
	}}
/>
