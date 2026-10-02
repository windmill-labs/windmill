<script lang="ts">
	import { untrack } from 'svelte'
	import { Plus, Trash2, X } from 'lucide-svelte'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from './Toggle.svelte'
	import {
		emptyQuestion,
		optionNameError,
		questionNameError,
		questionsFitRows,
		questionsToRows,
		rowsToQuestions,
		type DecisionQuestionType,
		type QuestionRow
	} from './flows/aiDecisionQuestions'
	import { getAiDecisionStep } from './flows/aiDecisionBranching'
	import AiDecisionQuestionRouting from './flows/content/AiDecisionQuestionRouting.svelte'

	interface Props {
		value: any
		disabled?: boolean
	}

	let { value = $bindable(), disabled = false }: Props = $props()

	const decisionStep = getAiDecisionStep()

	// Whether the cards can hold the value: TypeSafe also takes structured instructions and
	// descriptions, which the cards would flatten to text, so such questions stay in JSON.
	let fitsRows = $state(questionsFitRows(untrack(() => value)))
	let mode: 'form' | 'json' = $state(untrack(() => fitsRows) ? 'form' : 'json')
	let jsonCode = $state(
		JSON.stringify(
			untrack(() => value),
			null,
			2
		)
	)
	// The code editor reads `code` only when it is created, so a replaced value recreates it.
	let jsonEditorKey = $state(0)

	let rows: QuestionRow[] = $state(questionsToRows(untrack(() => value)))
	// What the rows last wrote or were read from. Rows hold drafts the value leaves out (an unnamed
	// question), so only an edit writes; opening the editor leaves the stored value untouched.
	let synced = JSON.stringify(rowsToQuestions(untrack(() => rows)))
	let written = JSON.stringify(untrack(() => value) ?? {})

	function readRows() {
		rows = questionsToRows(value)
		synced = JSON.stringify(rowsToQuestions(rows))
	}

	function readJson() {
		jsonCode = JSON.stringify(value, null, 2)
		jsonEditorKey++
	}

	$effect(() => {
		const questions = rowsToQuestions(rows)
		const json = JSON.stringify(questions)
		untrack(() => {
			if (mode === 'form' && json !== synced) {
				synced = json
				written = json
				value = questions
			}
		})
	})

	// A value replaced from elsewhere (an undo, the AI chat) is read into both views, and moves the
	// editor to JSON when the cards cannot hold it.
	$effect(() => {
		const json = JSON.stringify(value ?? {})
		untrack(() => {
			if (json !== written) {
				written = json
				fitsRows = questionsFitRows(value)
				if (!fitsRows) mode = 'json'
				readJson()
				readRows()
			}
		})
	})

	function switchTo(next: 'form' | 'json') {
		if (next === mode || (next === 'form' && !fitsRows)) return
		if (next === 'form') readRows()
		else readJson()
		mode = next
	}

	function addQuestion() {
		let i = rows.length + 1
		while (rows.some((r) => r.name === `question_${i}`)) i++
		rows.push(emptyQuestion(`question_${i}`))
	}

	const TYPES: { value: DecisionQuestionType; label: string }[] = [
		{ value: 'choice', label: 'Choice' },
		{ value: 'score', label: 'Score' },
		{ value: 'noul', label: 'Yes/no' }
	]
</script>

{#snippet label(text: string)}
	<span class="text-xs font-semibold text-emphasis">{text}</span>
{/snippet}

{#snippet error(text: string | undefined)}
	{#if text}
		<span class="text-2xs text-red-600 dark:text-red-400">{text}</span>
	{/if}
{/snippet}

<div class="flex flex-col gap-2 w-full">
	<div class="flex justify-end">
		<Toggle
			size="xs"
			lightMode
			disabled={disabled || !fitsRows}
			checked={mode === 'json'}
			options={{
				right: 'JSON',
				rightTooltip: fitsRows
					? 'Edit the questions as JSON'
					: 'These questions use structured instructions or descriptions, which only JSON can show'
			}}
			on:change={(e) => switchTo(e.detail ? 'json' : 'form')}
		/>
	</div>
	{#if mode === 'json'}
		<div class="w-full">
			{#key jsonEditorKey}
				{#await import('./JsonEditor.svelte') then Module}
					<Module.default
						bind:code={jsonCode}
						{disabled}
						on:changeValue={(e) => {
							written = JSON.stringify(e.detail ?? {})
							fitsRows = questionsFitRows(e.detail)
							value = e.detail
						}}
					/>
				{/await}
			{/key}
		</div>
	{:else}
		<div class="flex flex-col gap-3 w-full">
			{#each rows as row, i (i)}
				<div class="flex flex-col gap-3 rounded-md bg-surface-tertiary p-3 shadow-sm">
					<div class="flex items-start gap-2">
						<div class="flex flex-col gap-1 grow">
							{@render label('Name')}
							<TextInput
								size="sm"
								bind:value={row.name}
								error={questionNameError(rows, i)}
								inputProps={{ disabled, placeholder: 'intent' }}
							/>
							{@render error(questionNameError(rows, i))}
						</div>
						<div class="flex flex-col gap-1">
							{@render label('Answer')}
							<ToggleButtonGroup
								noWFull
								{disabled}
								selected={row.type}
								onSelected={(v) => (row.type = v as DecisionQuestionType)}
							>
								{#snippet children({ item })}
									{#each TYPES as t (t.value)}
										<ToggleButton value={t.value} label={t.label} {item} small />
									{/each}
								{/snippet}
							</ToggleButtonGroup>
						</div>
						<div class="pt-5">
							<Button
								variant="subtle"
								unifiedSize="sm"
								destructive
								iconOnly
								{disabled}
								startIcon={{ icon: Trash2 }}
								title="Delete question"
								onClick={() => rows.splice(i, 1)}
							/>
						</div>
					</div>

					<div class="flex flex-col gap-1">
						{@render label('Question')}
						<TextInput
							size="sm"
							underlyingInputEl="textarea"
							bind:value={row.instructions}
							autosizeParams={{ minHeight: 0 }}
							inputProps={{ disabled, placeholder: 'What does the customer want?', rows: 1 }}
						/>
					</div>

					{#if row.type === 'choice'}
						<div class="flex flex-col gap-1">
							{@render label('Options')}
							<span class="text-xs text-secondary"
								>What each option means helps the model choose</span
							>
							{#each row.options as option, j (j)}
								<div class="flex gap-2 items-start">
									<div class="w-1/3 shrink-0 flex flex-col gap-1">
										<TextInput
											size="sm"
											bind:value={option.name}
											error={optionNameError(row, j)}
											inputProps={{ disabled, placeholder: 'refund' }}
										/>
										{@render error(optionNameError(row, j))}
									</div>
									<TextInput
										size="sm"
										bind:value={option.description}
										inputProps={{ disabled, placeholder: 'What this option means' }}
									/>
									<Button
										variant="subtle"
										unifiedSize="sm"
										iconOnly
										disabled={disabled || row.options.length <= 2}
										startIcon={{ icon: X }}
										title="Remove option"
										onClick={() => row.options.splice(j, 1)}
									/>
								</div>
							{/each}
							<Button
								variant="subtle"
								unifiedSize="sm"
								wrapperClasses="self-start"
								{disabled}
								startIcon={{ icon: Plus }}
								onClick={() => row.options.push({ name: '', description: '' })}
							>
								Add option
							</Button>
						</div>
					{:else if row.type === 'score'}
						<div class="flex flex-col gap-1">
							{@render label('Levels')}
							<span class="text-xs text-secondary">Lowest first</span>
							{#each row.levels as _, j (j)}
								<div class="flex gap-2 items-center">
									<span class="text-xs text-secondary w-4 text-right">{j}</span>
									<TextInput
										size="sm"
										bind:value={row.levels[j]}
										inputProps={{ disabled, placeholder: j === 0 ? 'Can wait' : 'Right now' }}
									/>
									<Button
										variant="subtle"
										unifiedSize="sm"
										iconOnly
										disabled={disabled || row.levels.length <= 2}
										startIcon={{ icon: X }}
										title="Remove level"
										onClick={() => row.levels.splice(j, 1)}
									/>
								</div>
							{/each}
							<Button
								variant="subtle"
								unifiedSize="sm"
								wrapperClasses="self-start"
								disabled={disabled || row.levels.length >= 10}
								startIcon={{ icon: Plus }}
								onClick={() => row.levels.push('')}
							>
								Add level
							</Button>
						</div>
					{:else}
						<div class="grid grid-cols-2 gap-2">
							<div class="flex flex-col gap-1">
								{@render label('Yes means')}
								<TextInput size="sm" bind:value={row.yes} inputProps={{ disabled }} />
								<span class="text-2xs text-hint">Optional</span>
							</div>
							<div class="flex flex-col gap-1">
								{@render label('No means')}
								<TextInput size="sm" bind:value={row.no} inputProps={{ disabled }} />
								<span class="text-2xs text-hint">Optional</span>
							</div>
						</div>
					{/if}
					{#if decisionStep?.id && row.type !== 'score' && row.name.trim()}
						<AiDecisionQuestionRouting decisionId={decisionStep.id} question={row.name.trim()} />
					{/if}
				</div>
			{/each}
			<Button
				variant="default"
				unifiedSize="sm"
				wrapperClasses="self-start"
				{disabled}
				startIcon={{ icon: Plus }}
				onClick={addQuestion}
			>
				Add question
			</Button>
		</div>
	{/if}
</div>
