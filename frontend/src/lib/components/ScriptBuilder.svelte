<script lang="ts">
	import { Script, ScriptService } from '$lib/gen'

	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { inferArgs } from '$lib/infer'
	import { initialCode, isInitialCode } from '$lib/script_helpers'
	import { userStore, workspaceStore } from '$lib/stores'
	import { emptySchema, encodeState, sendUserToast, setQueryWithoutLoad } from '$lib/utils'
	import Path from './Path.svelte'
	import RadioButton from './RadioButton.svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import ScriptSchema from './ScriptSchema.svelte'
	import CenteredPage from './CenteredPage.svelte'
	import UnsavedConfirmationModal from './common/confirmationModal/UnsavedConfirmationModal.svelte'
	import { dirtyStore } from './common/confirmationModal/dirtyStore'
	import { Button } from './common'
	import { faChevronDown, faChevronUp, faPen, faSave } from '@fortawesome/free-solid-svg-icons'
	import Breadcrumb from './common/breadcrumb/Breadcrumb.svelte'
	import Toggle from './Toggle.svelte'
	import LanguageIcon from './common/languageIcons/LanguageIcon.svelte'
	import type { SupportedLanguage } from '$lib/common'

	export let script: Script
	export let initialPath: string = ''
	export let template: 'pgsql' | 'mysql' | 'script' = 'script'
	export let initialArgs: Record<string, any> = {}

	const langs: [string, SupportedLanguage][] = [
		['Typescript', Script.language.DENO],
		['Python', Script.language.PYTHON3],
		['Go', Script.language.GO],
		['Bash', Script.language.BASH]
	]
	let viewScriptKind = script.kind !== Script.kind.SCRIPT

	let pathError = ''

	let summaryC: HTMLInputElement | undefined = undefined
	let pathC: Path | undefined = undefined

	$: setQueryWithoutLoad($page.url, 'state', encodeState(script))
	$: step = Number($page.url.searchParams.get('step')) || 1

	if (script.content == '') {
		initContent(script.language, script.kind, template)
	}

	function initContent(
		language: 'deno' | 'python3' | 'go' | 'bash',
		kind: Script.kind,
		template: 'pgsql' | 'mysql' | 'script'
	) {
		script.content = initialCode(language, kind, template)
	}

	async function editScript(): Promise<void> {
		try {
			$dirtyStore = false
			localStorage.removeItem(script.path)

			script.schema = script.schema ?? emptySchema()
			if (!script.schema) {
				await inferArgs(script.language, script.content, script.schema)
			}
			const newHash = await ScriptService.createScript({
				workspace: $workspaceStore!,
				requestBody: {
					path: script.path,
					summary: script.summary,
					description: script.description ?? '',
					content: script.content,
					parent_hash: script.hash != '' ? script.hash : undefined,
					schema: script.schema,
					is_template: script.is_template,
					language: script.language,
					kind: script.kind
				}
			})
			sendUserToast(`Success! New script version created with hash ${newHash}`)
			goto(`/scripts/get/${newHash}`)
		} catch (error) {
			sendUserToast(`Impossible to save the script: ${error.body}`, true)
		}
	}

	async function changeStep(step: number) {
		if (step > 1) {
			script.schema = script.schema ?? emptySchema()
			await inferArgs(script.language, script.content, script.schema)
		}
		goto(`?step=${step}`)
	}
	$: kind = script.kind as 'script' | 'trigger' | 'approval' | undefined
</script>

{#if !$userStore?.operator}
	<UnsavedConfirmationModal />
	<div class="flex flex-col h-screen">
		<!-- Nav between steps-->
		<div class="flex flex-col w-full px-2 py-1  border-b shadow-sm">
			<div
				class="justify-between flex flex-row w-full items-center overflow-x-auto scrollbar-hidden px-2"
			>
				<div class="flex flex-row py-1">
					<Breadcrumb
						items={['Metadata', 'Code', 'Advanced']}
						selectedIndex={step}
						on:select={(e) => changeStep(e.detail.index + 1)}
						disabled={pathError != ''}
					>
						<svelte:fragment slot="separator">/</svelte:fragment>
					</Breadcrumb>
				</div>

				<div class="gap-1 flex-row hidden md:flex shrink overflow-hidden">
					<Button
						btnClasses="hidden lg:inline-flex"
						startIcon={{ icon: faPen }}
						variant="contained"
						color="light"
						size="xs"
						on:click={async () => {
							await changeStep(1)
							setTimeout(() => pathC?.focus(), 100)
						}}
					>
						{script.path}
					</Button>

					<Button
						startIcon={{ icon: faPen }}
						variant="contained"
						color="light"
						size="xs"
						on:click={async () => {
							await changeStep(1)
							setTimeout(() => summaryC?.focus(), 100)
						}}
					>
						<div class="max-w-[10em] !truncate">
							{script.summary == '' || !script.summary ? 'No summary' : script.summary}
						</div>
					</Button>
				</div>
				<div class="flex flex-row gap-x-2">
					<Button
						variant="border"
						size="sm"
						btnClasses={step == 1 ? 'hidden sm:invisible' : ''}
						on:click={() => changeStep(step - 1)}
					>
						Back
					</Button>
					<Button
						size="sm"
						variant={step == 1 ? 'contained' : 'border'}
						btnClasses={step == 3 ? 'invisible' : ''}
						disabled={step === 1 && pathError !== ''}
						on:click={() => changeStep(step + 1)}
					>
						Next
					</Button>
					<Button
						size="sm"
						variant={step == 1 ? 'border' : 'contained'}
						disabled={step === 1 && pathError !== ''}
						btnClasses={step == 1 && initialPath == '' ? 'invisible' : ''}
						startIcon={{ icon: faSave }}
						on:click={editScript}>Save</Button
					>
				</div>
			</div>
		</div>

		<!-- metadata -->
		{#if step === 1}
			<CenteredPage>
				<h2 class="border-b pb-1 mt-4 mb-2">Path</h2>
				<Path
					bind:this={pathC}
					bind:error={pathError}
					bind:path={script.path}
					{initialPath}
					on:enter={() => changeStep(2)}
					namePlaceholder="my_script"
					kind="script"
				/>
				<h2 class="border-b pb-1 mt-8 mb-4">Summary</h2>
				<input
					type="text"
					bind:this={summaryC}
					bind:value={script.summary}
					placeholder="A short summary of the script displayed when the script is listed"
				/>
				<h2 class="border-b pb-1 mt-8 mb-6">Language</h2>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each langs as [label, lang]}
						{@const isPicked = script.language == lang && template == 'script'}
						<Button
							size="sm"
							variant="border"
							color={isPicked ? 'blue' : 'dark'}
							btnClasses={isPicked ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
							on:click={() => {
								script.language = lang
								template = 'script'
								initContent(lang, script.kind, template)
							}}
						>
							<LanguageIcon {lang} /><span class="ml-2">{label}</span>
						</Button>
					{/each}
					<Button
						size="sm"
						variant="border"
						color={template == 'pgsql' ? 'blue' : 'dark'}
						btnClasses={template == 'pgsql' ? '!border-2 !bg-blue-50/75' : 'm-[1px]'}
						on:click={() => {
							script.language = Script.language.DENO
							template = 'pgsql'
							initContent(script.language, script.kind, template)
						}}
					>
						<LanguageIcon lang="pgsql" /><span class="ml-2">PostgreSQL</span>
					</Button>
					<Button
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
						<LanguageIcon lang="mysql" /><span class="ml-2">MySQL</span>
					</Button>
				</div>
				<h2 class="border-b pb-1 mt-8 mb-4">Advanced</h2>
				<div class="mb-4">
					<Button
						color="light"
						size="sm"
						endIcon={{ icon: viewScriptKind ? faChevronUp : faChevronDown }}
						on:click={() => (viewScriptKind = !viewScriptKind)}
					>
						Tag this script as having a specific purpose inside flows
					</Button>
				</div>
				{#if viewScriptKind}
					<div class="max-w-lg">
						<RadioButton
							label="Script Type"
							options={[
								['Action', Script.kind.SCRIPT],
								[
									{
										title: 'Trigger',
										desc: `First module of flows to trigger them based on watching changes external periodically using an internal state`
									},
									Script.kind.TRIGGER
								],
								[
									{
										title: 'Approval',
										desc: `Send notification externally to ask for approval to continue a flow`
									},
									Script.kind.APPROVAL
								],
								[
									{
										title: 'Error Handler',
										desc: `Handle errors for flows after all retries attempts have been exhausted`
									},
									Script.kind.FAILURE
								]

								// ['Command Handler', Script.kind.COMMAND]
							]}
							on:change={(e) => {
								if (isInitialCode(script.content)) {
									template = 'script'
									initContent(script.language, e.detail, template)
								}
							}}
							bind:value={script.kind}
						/>
					</div>
				{/if}
				<div class="ml-3">
					<Toggle
						bind:checked={script.is_template}
						options={{ right: 'Save as a workspace template' }}
					/>
				</div>
			</CenteredPage>
		{:else if step === 2}
			<ScriptEditor
				bind:schema={script.schema}
				path={script.path}
				bind:code={script.content}
				lang={script.language}
				{initialArgs}
				{kind}
			/>
		{:else if step === 3}
			<CenteredPage>
				<ScriptSchema bind:description={script.description} bind:schema={script.schema} />
			</CenteredPage>
		{/if}
	</div>
{:else}
	Script Builder not available to operators
{/if}
