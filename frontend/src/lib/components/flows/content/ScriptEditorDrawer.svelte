<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'
	import { ScriptService, type Preview } from '$lib/gen'
	import { inferArgs } from '$lib/infer'
	import { workspaceStore } from '$lib/stores'
	import { emptySchema, getScriptByPath, sendUserToast } from '$lib/utils'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import { Loader2 } from 'lucide-svelte'
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
						kind: script.kind
					}
				})
				callback?.()
			} catch (error) {
				sendUserToast(`Impossible to save the script: ${error.body}`, true)
			}
		}
	}
</script>

<Drawer bind:this={scriptEditorDrawer} size="1200px">
	<DrawerContent
		title="Script Editor"
		noPadding
		forceOverflowVisible
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
					fixedOverflowWidgets={false}
					bind:code={script.content}
					bind:schema={script.schema}
				/>
			{/key}
		{:else}
			<div
				out:fade={{ duration: 200 }}
				class="absolute inset-0 center-center flex-col bg-white text-gray-600 border"
			>
				<Loader2 class="animate-spin" size={16} />
				<span class="text-xs mt-1">Loading</span>
			</div>
		{/if}
		<svelte:fragment slot="actions">
			<Button
				on:click={async () => {
					await saveScript()
					dispatch('save')
					scriptEditorDrawer.closeDrawer()
				}}
				disabled={!script}
				startIcon={{ icon: faSave }}>Save</Button
			>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
