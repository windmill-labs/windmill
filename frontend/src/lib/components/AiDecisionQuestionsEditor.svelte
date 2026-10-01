<script lang="ts">
	import { untrack } from 'svelte'
	import { Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
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

	interface Props {
		value: any
		disabled?: boolean
	}

	let { value = $bindable(), disabled = false }: Props = $props()

	// A value the cards cannot hold is edited as JSON. Decided on open and again whenever the value
	// is replaced from elsewhere (an undo, the AI chat), never by an edit made here.
	let fitsRows = $state(questionsFitRows(untrack(() => value)))
	let jsonCode = $state(
		JSON.stringify(
			untrack(() => value),
			null,
			2
		)
	)

	let rows: QuestionRow[] = $state(questionsToRows(untrack(() => value)))
	// What the rows last wrote or were read from. Rows hold drafts the value leaves out (an unnamed
	// question), so only an edit writes; opening the editor leaves the stored value untouched.
	let synced = JSON.stringify(rowsToQuestions(untrack(() => rows)))
	let written = JSON.stringify(untrack(() => value) ?? {})

	$effect(() => {
		const questions = rowsToQuestions(rows)
		const json = JSON.stringify(questions)
		untrack(() => {
			if (fitsRows && json !== synced) {
				synced = json
				written = json
				value = questions
			}
		})
	})

	$effect(() => {
		const json = JSON.stringify(value ?? {})
		untrack(() => {
			if (json !== written) {
				written = json
				fitsRows = questionsFitRows(value)
				jsonCode = JSON.stringify(value, null, 2)
				rows = questionsToRows(value)
				synced = JSON.stringify(rowsToQuestions(rows))
			}
		})
	})

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

{#if !fitsRows}
	<div class="flex flex-col gap-1 w-full">
		<span class="text-xs text-secondary">
			These questions use structured instructions or descriptions, so they are edited as JSON.
		</span>
		{#await import('./JsonEditor.svelte') then Module}
			<Module.default
				bind:code={jsonCode}
				{disabled}
				on:changeValue={(e) => {
					written = JSON.stringify(e.detail ?? {})
					value = e.detail
				}}
			/>
		{/await}
	</div>
{:else}
	<div class="flex flex-col gap-3 w-full">
		{#each rows as row, i (i)}
			<div class="flex flex-col gap-2 border rounded-md p-3 bg-surface">
				<div class="flex items-start gap-2">
					<div class="flex flex-col gap-1 grow">
						<span class="text-xs text-secondary">Name</span>
						<TextInput
							size="sm"
							bind:value={row.name}
							error={questionNameError(rows, i)}
							inputProps={{ disabled, placeholder: 'intent' }}
						/>
					</div>
					<div class="flex flex-col gap-1">
						<span class="text-xs text-secondary">Answer</span>
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
							startIcon={{ icon: X }}
							title="Remove question"
							onClick={() => rows.splice(i, 1)}
						/>
					</div>
				</div>
				{#if questionNameError(rows, i)}
					<span class="text-2xs text-red-500">{questionNameError(rows, i)}</span>
				{/if}

				<div class="flex flex-col gap-1">
					<span class="text-xs text-secondary">Question</span>
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
						<span class="text-xs text-secondary">Options</span>
						{#each row.options as option, j (j)}
							<div class="flex gap-2 items-start">
								<div class="w-1/3 shrink-0">
									<TextInput
										size="sm"
										bind:value={option.name}
										error={optionNameError(row, j)}
										inputProps={{ disabled, placeholder: 'refund', title: optionNameError(row, j) }}
									/>
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
						<span class="text-xs text-secondary">Levels, lowest first</span>
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
							<span class="text-xs text-secondary">Yes means (optional)</span>
							<TextInput size="sm" bind:value={row.yes} inputProps={{ disabled }} />
						</div>
						<div class="flex flex-col gap-1">
							<span class="text-xs text-secondary">No means (optional)</span>
							<TextInput size="sm" bind:value={row.no} inputProps={{ disabled }} />
						</div>
					</div>
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
