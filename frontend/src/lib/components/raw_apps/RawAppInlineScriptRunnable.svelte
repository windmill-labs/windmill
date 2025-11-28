<script lang="ts">
	import EmptyInlineScript from '../apps/editor/inlineScriptsPanel/EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from '../apps/editor/inlineScriptsPanel/InlineScriptRunnableByPath.svelte'
	import {
		isRunnableByName,
		isRunnableByPath,
		type InlineScript,
		type RunnableWithFields,
		type StaticAppInput,
		type UserAppInput
	} from '../apps/inputType'
	import { createEventDispatcher } from 'svelte'
	import RawAppInlineScriptEditor from './RawAppInlineScriptEditor.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import Tabs from '../common/tabs/Tabs.svelte'
	import { Tab } from '../common'
	import RawAppInputsSpecEditor from './RawAppInputsSpecEditor.svelte'
	import SplitPanesWrapper from '../splitPanes/SplitPanesWrapper.svelte'
	import SchemaForm from '../SchemaForm.svelte'
	import RunnableJobPanelInner from '../apps/editor/RunnableJobPanelInner.svelte'
	import JobLoader from '../JobLoader.svelte'
	import type { Job, ScriptLang } from '$lib/gen'

	type RunnableWithInlineScript = RunnableWithFields & {
		inlineScript?: InlineScript & { language: ScriptLang }
	}
	export type Runnable = RunnableWithInlineScript | undefined
	interface Props {
		runnable: Runnable
		id: string
		appPath: string
		lastDeployedCode?: string | undefined
	}

	let { runnable = $bindable(), id, appPath }: Props = $props()

	const dispatch = createEventDispatcher()

	async function fork(nrunnable: Runnable) {
		runnable = nrunnable == undefined ? undefined : { ...runnable, ...nrunnable }
	}

	function onPick(o: { runnable: Runnable; fields: Record<string, StaticAppInput> }) {
		runnable =
			o.runnable == undefined
				? undefined
				: {
						...(runnable ?? {}),
						...o.runnable,
						fields: o.fields
					}
	}

	let selectedTab = $state('inputs')
	let args = $state({})

	function getSchema(runnable: RunnableWithFields) {
		if (isRunnableByPath(runnable)) {
			return runnable.schema
		} else if (isRunnableByName(runnable) && runnable.inlineScript) {
			return runnable.inlineScript.schema
		}
		return {}
	}

	let jobLoader: JobLoader | undefined = $state()
	let testJob: Job | undefined = $state()
	let testIsLoading = $state(false)
	let scriptProgress = $state(0)

	function onFieldsChange(fields: Record<string, StaticAppInput | UserAppInput>) {
		if (args == undefined) {
			args = {}
		}
		Object.entries(fields ?? {}).forEach(([k, v]) => {
			if (v.type == 'static') {
				args[k] = v.value
			}
		})
	}

	async function testPreview() {
		selectedTab = 'test'
		if (isRunnableByName(runnable)) {
			await jobLoader?.runPreview(
				appPath + '/' + id,
				runnable.inlineScript?.content ?? '',
				runnable.inlineScript?.language,
				args,
				undefined
			)
		} else if (isRunnableByPath(runnable)) {
			if (jobLoader && isRunnableByPath(runnable)) {
				if (runnable.runType == 'flow') {
					await jobLoader.runFlowByPath(runnable.path, args)
				} else if (runnable.runType == 'script' || runnable.runType == 'hubscript') {
					await jobLoader.runScriptByPath(runnable.path, args)
				}
			}
		}
	}
	$effect(() => {
		onFieldsChange(runnable?.fields ?? {})
	})
</script>

<JobLoader
	noCode={true}
	bind:scriptProgress
	bind:this={jobLoader}
	bind:isLoading={testIsLoading}
	bind:job={testJob}
/>

{#if isRunnableByPath(runnable) || (isRunnableByName(runnable) && runnable.inlineScript)}
	<Splitpanes>
		<Pane size={55}>
			{#if isRunnableByName(runnable)}
				<RawAppInlineScriptEditor
					on:createScriptFromInlineScript={() => dispatch('createScriptFromInlineScript', runnable)}
					{id}
					bind:inlineScript={runnable.inlineScript}
					bind:name={runnable.name}
					bind:fields={runnable.fields}
					isLoading={testIsLoading}
					onRun={testPreview}
					onCancel={async () => {
						if (jobLoader) {
							await jobLoader.cancelJob()
						}
					}}
					on:delete
					path={appPath}
				/>
			{:else if isRunnableByPath(runnable)}
				<InlineScriptRunnableByPath
					rawApps
					bind:runnable
					bind:fields={runnable.fields}
					on:fork={(e) => fork(e.detail)}
					on:delete
					{id}
					isLoading={testIsLoading}
					onRun={testPreview}
					onCancel={async () => {
						if (jobLoader) {
							await jobLoader.cancelJob()
						}
					}}
				/>
			{/if}
		</Pane>
		<Pane size={45}>
			<Tabs bind:selected={selectedTab}>
				<Tab value="inputs" label="Inputs" />
				<Tab value="test" label="Test" />
				{#snippet content()}
					{#if selectedTab == 'inputs'}
						{#if runnable?.fields}
							<div class="w-full flex flex-col gap-4 p-2">
								{#each Object.keys(runnable.fields) as k}
									{@const meta = runnable.fields[k]}
									<RawAppInputsSpecEditor
										key={k}
										bind:componentInput={runnable.fields[k]}
										{id}
										shouldCapitalize
										fieldType={meta?.['fieldType']}
										subFieldType={meta?.['subFieldType']}
										format={meta?.['format']}
										selectOptions={meta?.['selectOptions']}
										tooltip={meta?.['tooltip']}
										placeholder={meta?.['placeholder']}
										customTitle={meta?.['customTitle']}
										loading={meta?.['loading']}
										documentationLink={meta?.['documentationLink']}
										markdownTooltip={meta?.['markdownTooltip']}
										allowTypeChange={meta?.['allowTypeChange']}
										displayType
									/>
								{/each}
							</div>
						{:else}
							<div class="text-primary text-xs">No inputs</div>
						{/if}
					{:else if selectedTab == 'test'}
						<SplitPanesWrapper>
							<Splitpanes horizontal class="grow">
								<Pane size={50}>
									<div class="px-2 py-3 h-full overflow-auto">
										<SchemaForm
											on:keydownCmdEnter={testPreview}
											disabledArgs={Object.entries(runnable?.fields ?? {})
												.filter(([_, v]) => v.type == 'static')
												.map(([k]) => k)}
											schema={runnable ? getSchema(runnable) : {}}
											bind:args
										/>
									</div>
								</Pane>
								<Pane size={50}>
									<RunnableJobPanelInner frontendJob={false} {testJob} {testIsLoading} />
								</Pane>
							</Splitpanes>
						</SplitPanesWrapper>
					{/if}
				{/snippet}
			</Tabs>
		</Pane>
	</Splitpanes>
{:else}
	<EmptyInlineScript
		unusedInlineScripts={[]}
		rawApps
		on:pick={(e) =>
			onPick(e.detail as { runnable: Runnable; fields: Record<string, StaticAppInput> })}
		on:delete
		showScriptPicker
		on:new={(e) => {
			runnable = {
				type: 'inline',
				inlineScript: e.detail,
				name: runnable?.name ?? 'Background Runnable'
			}
		}}
	/>
{/if}
