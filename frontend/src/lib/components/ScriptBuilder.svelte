<script lang="ts">
	import { Script, ScriptService } from '$lib/gen'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode } from '$lib/script_helpers'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, encodeState, sendUserToast, setQueryWithoutLoad } from '$lib/utils'
	import Path from './Path.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import { Badge, Button, Drawer } from './common'
	import { faSave } from '@fortawesome/free-solid-svg-icons'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage } from '$lib/common'
	import Tooltip from './Tooltip.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import { Pen } from 'lucide-svelte'
	import autosize from 'svelte-autosize'
	import type Editor from './Editor.svelte'

	export let script: Script
	export let initialPath: string = ''
	export let template: 'pgsql' | 'mysql' | 'script' = 'script'
	export let initialArgs: Record<string, any> = {}
	export let lockedLanguage = false
	export let topHash: string | undefined = undefined

	let metadataOpen = initialPath == '' && $page.url.searchParams.get('state') == undefined
	let advancedOpen = false

	let editor: Editor | undefined = undefined
	let scriptEditor: ScriptEditor | undefined = undefined

	export function setCode(code: string): void {
		editor?.setCode(code)
	}

	const langs: [string, SupportedLanguage][] = [
		['Typescript', Script.language.DENO],
		['Python', Script.language.PYTHON3],
		['Go', Script.language.GO],
		['Bash', Script.language.BASH]
	]
	const scriptKindOptions: { value: Script.kind; title: string; desc?: string }[] = [
		{
			value: Script.kind.SCRIPT,
			title: 'Action'
		},
		{
			value: Script.kind.TRIGGER,
			title: 'Trigger',
			desc: 'First module of flows to trigger them based on external changes. These kind of scripts are usually running on a schedule to periodically look for changes.'
		},
		{
			value: Script.kind.APPROVAL,
			title: 'Approval',
			desc: 'Send notifications externally to ask for approval to continue a flow.'
		},
		{
			value: Script.kind.FAILURE,
			title: 'Error Handler',
			desc: 'Handle errors in flows after all retry attempts have been exhausted.'
		}
	]

	let pathError = ''
	let loadingSave = false

	$: setQueryWithoutLoad($page.url, [{ key: 'state', value: encodeState(script) }])

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	function initContent(
		language: 'deno' | 'python3' | 'go' | 'bash',
		kind: Script.kind,
		template: 'pgsql' | 'mysql' | 'script'
	) {
		script.content = initialCode(language, kind, template)
		scriptEditor?.inferSchema(script.content, language)
	}

	async function editScript(leave: boolean): Promise<void> {
		loadingSave = true
		try {
			$dirtyStore = false
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			try {
				await inferArgs(script.language, script.content, script.schema)
			} catch (error) {
				sendUserToast(
					`The main signature was not parsable. This script is considered to be without main function`
				)
			}

			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: script.hash != '' ? topHash ?? script.hash : undefined,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind
				}
			})
			if (leave) {
				history.replaceState(history.state, '', `/scripts/edit/${newHash}`)
				goto(`/scripts/get/${newHash}?workspace_id=${$workspaceStore}`)
			} else {
				await goto(`/scripts/edit/${newHash}`)
				script.hash = newHash
				topHash = undefined
			}
		} catch (error) {
			sendUserToast(`Error while saving the script: ${error.body || error.message}`, true)
		}
		loadingSave = false
	}

	const dropdownItems: Array<{ label: string; onClick: () => void }> = [
		{
			label: 'Save and leave',
			onClick: () => editScript(true)
		}
	]

	if (initialPath != '') {
		dropdownItems.push({
			label: 'Fork',
			onClick: () => {
				window.open(`/scripts/add?template=${initialPath}`)
			}
		})
	}
</script>

{#if !$userStore?.operator}
	<Drawer placement="right" open={metadataOpen} size="800px">
		<DrawerContent title="Metadata" on:close={() => (metadataOpen = false)}>
			<h2 class="border-b pb-1 mb-4">Path</h2>
			<Path
				bind:error={pathError}
				bind:path={script.path}
				{initialPath}
				namePlaceholder="script"
				kind="script"
			/>
			<h2 class="border-b pb-1 mt-10 mb-4">Summary</h2>

			<input
				type="text"
				bind:value={script.summary}
				placeholder="Short summary to be displayed when listed"
			/>
			<h2 class="border-b pb-1 mt-10 mb-4">Language</h2>

			{#if lockedLanguage}
				<div class="text-sm text-gray-600 italic mb-2">
					As a forked script, the language '{script.language}' cannot be modified.
				</div>
			{/if}
			<div class="flex flex-row gap-2 flex-wrap">
				{#each langs as [label, lang]}
					{@const isPicked = script.language == lang && template == 'script'}
					<Button
						size="sm"
						variant="border"
						color={isPicked ? 'blue' : 'dark'}
						btnClasses={isPicked ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							template = 'script'
							initContent(lang, script.kind, template)
							script.language = lang
						}}
						disabled={lockedLanguage}
					>
						<LanguageIcon {lang} />
						<span class="ml-2 py-2">{label}</span>
					</Button>
				{/each}
				<Button
					size="sm"
					variant="border"
					color={template == 'pgsql' ? 'blue' : 'dark'}
					btnClasses={template == 'pgsql' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
					disabled={lockedLanguage}
					on:click={() => {
						template = 'pgsql'
						initContent(script.language, script.kind, template)
						script.language = Script.language.DENO
					}}
				>
					<LanguageIcon lang="pgsql" /><span class="ml-2 py-2">PostgreSQL</span>
				</Button>
				<!-- <Button
					size="sm"
					variant="border"
					color={template == 'mysql' ? 'blue' : 'dark'}
					btnClasses={template == 'mysql' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
					on:click={() => {
						script.language = Script.language.DENO
						template = 'mysql'
						initContent(script.language, script.kind, template)
					}}
				>
					<LanguageIcon lang="mysql" /><span class="ml-2 py-2">MySQL</span>
				</Button> -->
			</div>

			<h2 class="border-b pb-1 mt-10 mb-4">Description</h2>
			<textarea
				use:autosize
				bind:value={script.description}
				placeholder="Edit description"
				class="text-sm"
			/>
		</DrawerContent>
	</Drawer>

	<div class="flex flex-col h-screen">
		<div class="flex flex-col w-full px-2 py-1 border-b shadow-sm">
			<div class="justify-between flex gap-8 w-full items-center px-2">
				<div class="min-w-64 w-full max-w-md">
					<input
						type="text"
						placeholder="Script summary"
						class="text-sm w-full font-semibold"
						bind:value={script.summary}
					/>
				</div>
				<div class="gap-4 flex">
					<div class="flex justify-start w-full">
						<div>
							<button
								on:click={async () => {
									metadataOpen = true
								}}
							>
								<Badge
									color="gray"
									class="center-center !bg-gray-300 !text-gray-600 !h-[28px]  !w-[70px] rounded-r-none"
								>
									<Pen size={12} class="mr-2" /> Path
								</Badge>
							</button>
						</div>
						<input
							type="text"
							readonly
							value={script.path}
							size={script.path?.length || 50}
							class="font-mono !text-xs !min-w-[96px] !max-w-[300px] !w-full !h-[28px] !my-0 !py-0 !border-l-0 !rounded-l-none"
							on:focus={({ currentTarget }) => {
								currentTarget.select()
							}}
						/>
					</div>
				</div>
				<div class="center-center">
					<button
						on:click={async () => {
							metadataOpen = true
						}}
					>
						<LanguageIcon lang={script.language} />
					</button>
				</div>
				<div class="flex flex-row gap-x-4">
					<Button
						color="dark"
						variant="border"
						size="xs"
						on:click={() => {
							advancedOpen = true
						}}
					>
						Customise
					</Button>
					<Button
						color="dark"
						loading={loadingSave}
						size="sm"
						startIcon={{ icon: faSave }}
						on:click={() => editScript(false)}
						{dropdownItems}
					>
						Save
					</Button>
				</div>
			</div>
		</div>

		<Drawer open={advancedOpen} size="800px">
			<DrawerContent title="Customise" on:close={() => (advancedOpen = false)}>
				<h2 class="border-b pb-1 mb-4"
					>Script Kind &nbsp;<Tooltip
						>Tag this script's purpose within flows such that it is available as the corresponding
						action.</Tooltip
					></h2
				>
				<div class="flex flex-wrap gap-2">
					{#each scriptKindOptions as { value, title, desc }}
						{@const isPicked = script.kind === value}
						<Button
							size="sm"
							variant="border"
							color={isPicked ? 'blue' : 'dark'}
							btnClasses="font-medium {isPicked ? '!bg-blue-50/75' : ''}"
							on:click={() => {
								template = 'script'
								script.kind = value
								initContent(script.language, value, template)
								setCode(script.content)
							}}
						>
							{title}
							{#if desc}
								<Tooltip class="mb-0.5 ml-1">
									{desc}
								</Tooltip>
							{/if}
						</Button>
					{/each}
				</div>
				<h2 class="border-b pb-1 mt-10 mb-4"
					>Arguments &nbsp;<Tooltip
						>The arguments are synced with the main signature but you may refine the parts that
						cannot be inferred from the type directly.</Tooltip
					></h2
				>
				<ScriptSchema bind:schema={script.schema} />
			</DrawerContent>
		</Drawer>
		<ScriptEditor
			bind:editor
			bind:this={scriptEditor}
			bind:schema={script.schema}
			path={script.path}
			bind:code={script.content}
			lang={script.language}
			{initialArgs}
			kind={script.kind}
		/>
	</div>
{:else}
	Script Builder not available to operators
{/if}
