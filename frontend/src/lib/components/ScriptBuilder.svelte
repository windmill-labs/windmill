<script lang="ts">
	import { ScriptService, type Script } from '$lib/gen'

	import { emptySchema, pathIsEmpty, sendUserToast } from '$lib/utils'
	import { onDestroy } from 'svelte'
	import ScriptEditor from './ScriptEditor.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'
	import Path from './Path.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { workspaceStore } from '$lib/stores'
	import ScriptSchema from './ScriptSchema.svelte'
	import { inferArgs } from '$lib/infer'
	import Required from './Required.svelte'
	import RadioButton from './RadioButton.svelte'
	import { DENO_INIT_CODE, PYTHON_INIT_CODE } from '$lib/script_helpers'

	let editor: ScriptEditor
	let scriptSchema: ScriptSchema
	$: step = Number($page.url.searchParams.get('step')) || 1

	export let script: Script
	export let initialPath: string = ''

	let pathError = ''

	$: {
		$page.url.searchParams.set('state', btoa(JSON.stringify(script)))
		history.replaceState({}, '', $page.url)
	}

	if (script.content == '') {
		initContent(script.language)
	}
	function initContent(lang: string) {
		script.content = lang == 'deno' ? DENO_INIT_CODE : PYTHON_INIT_CODE
	}

	async function editScript(): Promise<void> {
		try {
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
					language: script.language
				}
			})
			sendUserToast(`Success! New script version created with hash ${newHash}`)
			goto(`/scripts/get/${newHash}`)
		} catch (error) {
			sendUserToast(`Impossible to save the script: ${error.body}`, true)
		}
	}

	export function setCode(script: Script) {
		editor?.getEditor().setCode(script.content)

		if (scriptSchema) {
			if (script.schema) {
				scriptSchema.setSchema(script.schema)
			} else {
				scriptSchema.setSchema(emptySchema())
			}
		}
	}

	async function inferSchema() {
		await inferArgs(script.language, script.content, script.schema)
	}

	async function changeStep(step: number) {
		if (step == 3) {
			script.content = editor?.getEditor().getCode() ?? script.content
			await inferSchema()
			script.schema = script.schema
		}
		goto(`?step=${step}`)
	}

	onDestroy(() => {
		editor?.$destroy()
	})
</script>

<div class="flex flex-col h-screen max-w-screen-lg xl:-ml-20 xl:pl-4 w-full -mt-4 pt-4 md:mx-10 ">
	<!-- Nav between steps-->
	<div class="flex flex-col w-full">
		<div class="justify-between flex flex-row drop-shadow-sm w-full">
			<div class="wizard-nav flex flex-row w-full">
				<button
					class="{step === 1
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(1)
					}}>Step 1: Metadata</button
				>
				<button
					disabled={pathError != ''}
					class="{step === 2
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(2)
					}}>Step 2: Code</button
				>
				<button
					disabled={pathError != ''}
					class="{step === 3
						? 'default-button-disabled text-gray-700'
						: 'default-button-secondary'} min-w-max ml-2"
					on:click={() => {
						changeStep(3)
					}}>Step 3: UI customisation</button
				>
			</div>
			<div class="flex flex-row-reverse ml-2">
				{#if step != 3}
					<button
						disabled={step == 1 && pathError != ''}
						class="default-button px-6 max-h-8"
						on:click={() => {
							changeStep(step + 1)
						}}>Next</button
					>
					{#if step == 2}
						<button
							class="default-button-secondary px-6 max-h-8 mr-2"
							on:click={async () => {
								await inferSchema()
								editScript()
							}}>Save (commit)</button
						>
					{/if}
				{:else}
					<button class="default-button px-6 self-end" on:click={editScript}>Save (commit)</button>
				{/if}
			</div>
		</div>
		<div class="flex flex-row-reverse">
			<span class="my-1 text-sm text-gray-500 italic">
				{#if script.hash != ''} Editing from {script.hash} with path{/if}
				{#if initialPath && initialPath != script.path} {initialPath} &rightarrow; {/if}
				{script.path}
			</span>
		</div>
	</div>

	<!-- metadata -->
	{#if step === 1}
		<div class="grid grid-cols-1 gap-6 max-w-7xl">
			<Path
				bind:error={pathError}
				bind:path={script.path}
				{initialPath}
				namePlaceholder="example/my/script"
				kind="script"
			>
				<div slot="ownerToolkit" class="text-gray-700 text-2xs">
					Script permissions depend on their path. Select the group <span class="font-mono"
						>all</span
					>
					to share your script, and <span class="font-mono">user</span> to keep it private.
					<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
				</div>
			</Path>
			<h3 class="text-gray-700 pb-1 border-b">Language</h3>
			<div class="max-w-md">
				<RadioButton
					label="Language"
					small={true}
					options={[
						['Python 3.10', 'python3'],
						['Typescript (Deno)', 'deno']
					]}
					on:change={(e) => initContent(e.detail)}
					bind:value={script.language}
				/>
			</div>
			<h3 class="text-gray-700 pb-1 border-b">Metadata</h3>

			<label class="block ">
				<span class="text-gray-700">Summary <Required required={false} /></span>
				<textarea
					bind:value={script.summary}
					class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
					placeholder="A very short summary of the script displayed when the script is listed"
					rows="1"
				/>
			</label>
			<label class="block ">
				<span class="text-gray-700"
					>Description<Required required={false} detail="accept markdown formatting" />
					<textarea
						bind:value={script.description}
						class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
						placeholder="A description to help users understand what this script does and how to use it."
						rows="3"
					/>
				</span></label
			>
			<label class="block">
				<span class="text-gray-700 mr-2">Save as template</span>
				<input type="checkbox" bind:checked={script.is_template} />
			</label>

			<div>
				<h3 class="text-gray-700 ">Description rendered</h3>
				<div
					class="prose mt-5 text-xs shadow-inner shadow-blue p-4 overflow-auto"
					style="max-height: 200px;"
				>
					<SvelteMarkdown source={script.description ?? ''} />
				</div>
			</div>
		</div>
	{:else if step === 2}
		<div class="flex-1 overflow-auto">
			<ScriptEditor
				bind:this={editor}
				bind:schema={script.schema}
				path={script.path}
				bind:code={script.content}
				lang={script.language}
			/>
		</div>
	{:else if step === 3}
		<ScriptSchema
			bind:summary={script.summary}
			bind:description={script.description}
			bind:schema={script.schema}
		/>
	{/if}
</div>

<style>
	/* .wizard-nav {
		@apply w-1/2 sm:w-1/4;
	} */

	.wizard-nav button {
		max-height: 30px;
	}
</style>
