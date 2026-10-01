/** The questions of an AI decision as TypeSafe's Jev reads them, keyed by question name. */
export type DecisionQuestionType = 'choice' | 'score' | 'noul'

/** One question as the editor holds it: every type's criteria at once, so switching type and back
 *  keeps what was typed. Only the current type's are written out. */
export type QuestionRow = {
	name: string
	type: DecisionQuestionType
	instructions: string
	/** choice: each option and what it means */
	options: { name: string; description: string }[]
	/** score: the levels, lowest first */
	levels: string[]
	/** noul: what yes and no mean, both optional */
	yes: string
	no: string
}

export function emptyQuestion(name: string): QuestionRow {
	return {
		name,
		type: 'choice',
		instructions: '',
		options: [
			{ name: '', description: '' },
			{ name: '', description: '' }
		],
		levels: ['', ''],
		yes: '',
		no: ''
	}
}

export function questionsToRows(value: unknown): QuestionRow[] {
	if (!value || typeof value !== 'object' || Array.isArray(value)) {
		return []
	}
	return Object.entries(value as Record<string, any>).map(([name, q]) => {
		const row = emptyQuestion(name)
		row.type = q?.type === 'score' || q?.type === 'noul' ? q.type : 'choice'
		row.instructions = typeof q?.instructions === 'string' ? q.instructions : ''
		const criteria = q?.criteria
		if (
			row.type === 'choice' &&
			criteria &&
			typeof criteria === 'object' &&
			!Array.isArray(criteria)
		) {
			row.options = Object.entries(criteria).map(([name, description]) => ({
				name,
				description: typeof description === 'string' ? description : ''
			}))
		} else if (row.type === 'score' && Array.isArray(criteria)) {
			row.levels = criteria.map((l) => (typeof l === 'string' ? l : ''))
		} else if (row.type === 'noul' && criteria && typeof criteria === 'object') {
			row.yes = typeof criteria.true === 'string' ? criteria.true : ''
			row.no = typeof criteria.false === 'string' ? criteria.false : ''
		}
		return row
	})
}

/** Rows without a name are left out, as are options without a name: neither can be keyed. */
export function rowsToQuestions(rows: QuestionRow[]): Record<string, any> {
	const questions: Record<string, any> = {}
	for (const row of rows) {
		const name = row.name.trim()
		if (!name) continue
		let criteria: any
		if (row.type === 'choice') {
			criteria = Object.fromEntries(
				row.options.filter((o) => o.name.trim()).map((o) => [o.name.trim(), o.description])
			)
		} else if (row.type === 'score') {
			criteria = row.levels
		} else {
			criteria = {
				...(row.yes ? { true: row.yes } : {}),
				...(row.no ? { false: row.no } : {})
			}
			if (Object.keys(criteria).length === 0) criteria = undefined
		}
		questions[name] = {
			type: row.type,
			instructions: row.instructions,
			...(criteria !== undefined ? { criteria } : {})
		}
	}
	return questions
}

/** The name the editor should flag on a row, if any: TypeSafe keys questions and options by name,
 *  so an empty or repeated one would be dropped or overwrite another. */
export function questionNameError(rows: QuestionRow[], index: number): string | undefined {
	const name = rows[index].name.trim()
	if (!name) return 'Name the question'
	if (rows.findIndex((r) => r.name.trim() === name) !== index)
		return 'Another question has this name'
	return undefined
}

export function optionNameError(row: QuestionRow, index: number): string | undefined {
	const name = row.options[index].name.trim()
	if (name && row.options.findIndex((o) => o.name.trim() === name) !== index) {
		return 'Another option has this name'
	}
	return undefined
}

/** The options of each statically set choice question, in order. */
export function choiceQuestionOptions(value: unknown): { name: string; options: string[] }[] {
	return questionsToRows(value)
		.filter((r) => r.type === 'choice')
		.map((r) => ({
			name: r.name,
			options: r.options.map((o) => o.name.trim()).filter(Boolean)
		}))
}
