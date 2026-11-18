export type RunsColumnId =
	| 'started'
	| 'duration'
	| 'path'
	| 'labels'
	| 'triggered_by'
	| 'trigger_kind'
	| 'trigger_path'
	| 'run_as'
	| 'tag'
	| 'worker'
	| 'id'

export type RunsColumn = {
	id: RunsColumnId
	label: string
	width: string
	defaultVisible: boolean
	// The header's job count overflows into the started column, so it always stays.
	required?: boolean
	description?: string
}

// Display order of the table. The status and link columns are always shown.
export const RUNS_COLUMNS: RunsColumn[] = [
	{
		id: 'started',
		label: 'Started',
		width: 'minmax(100px, 0.8fr)',
		defaultVisible: true,
		required: true
	},
	{ id: 'duration', label: 'Duration', width: 'minmax(60px, 0.5fr)', defaultVisible: true },
	{ id: 'path', label: 'Path', width: 'minmax(150px, 2fr)', defaultVisible: true },
	{
		id: 'labels',
		label: 'Labels',
		width: 'minmax(120px, 1.3fr)',
		defaultVisible: true,
		description: 'Shown when at least one listed run has labels'
	},
	{
		id: 'triggered_by',
		label: 'Triggered by',
		width: 'minmax(110px, 1.5fr)',
		defaultVisible: true
	},
	{
		id: 'trigger_kind',
		label: 'Trigger kind',
		width: 'minmax(100px, 0.7fr)',
		defaultVisible: false
	},
	{
		id: 'trigger_path',
		label: 'Trigger',
		width: 'minmax(120px, 1.3fr)',
		defaultVisible: false,
		description: 'Path of the schedule, trigger or app that started the run'
	},
	{
		id: 'run_as',
		label: 'Run as',
		width: 'minmax(100px, 1fr)',
		defaultVisible: false,
		description: 'The identity the run is permissioned as'
	},
	{
		id: 'tag',
		label: 'Tag',
		width: 'minmax(70px, 0.5fr)',
		defaultVisible: true,
		description: 'Hidden when the table is narrow'
	},
	{ id: 'worker', label: 'Worker', width: 'minmax(90px, 0.8fr)', defaultVisible: false },
	{ id: 'id', label: 'Run ID', width: 'minmax(90px, 1fr)', defaultVisible: false }
]

export const DEFAULT_RUNS_COLUMNS: RunsColumnId[] = RUNS_COLUMNS.filter(
	(c) => c.defaultVisible
).map((c) => c.id)

export function normalizeRunsColumns(columns: unknown): RunsColumnId[] {
	const known = Array.isArray(columns)
		? RUNS_COLUMNS.filter((c) => columns.includes(c.id)).map((c) => c.id)
		: DEFAULT_RUNS_COLUMNS
	return [...new Set([...RUNS_COLUMNS.filter((c) => c.required).map((c) => c.id), ...known])]
}

function gridTracks(columns: RunsColumnId[], selectionMode: boolean): string[] {
	const widths = RUNS_COLUMNS.filter((c) => columns.includes(c.id)).map((c) => c.width)
	return [...(selectionMode ? ['50px'] : []), '60px', ...widths, 'minmax(40px, 0.3fr)']
}

export function runsGridTemplateColumns(columns: RunsColumnId[], selectionMode: boolean): string {
	return gridTracks(columns, selectionMode).join(' ')
}

// The rows are virtualized and clip what overflows them, so the table itself must be wide
// enough for every track's minimum and let its container scroll horizontally instead.
export function runsTableMinWidth(columns: RunsColumnId[], selectionMode: boolean): number {
	const tracksMin = gridTracks(columns, selectionMode).reduce(
		(sum, track) => sum + Number(/(\d+)px/.exec(track)?.[1] ?? 0),
		0
	)
	return Math.max(650, tracksMin + 16)
}
