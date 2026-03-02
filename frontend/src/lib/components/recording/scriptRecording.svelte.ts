import type { Job } from '$lib/gen'
import type { ActiveRecording, ActiveReplayData, RecordedJob, ScriptRecording } from './types'

export function createScriptRecording(): ScriptRecordingStore {
	let active = $state(false)
	let startTime = 0
	let scriptPath = ''
	let code = ''
	let language = ''
	let scriptArgs: Record<string, any> = {}
	let recordedJob: RecordedJob | undefined = undefined

	return {
		get active() {
			return active
		},
		start(path: string, scriptCode: string, lang: string, args: Record<string, any>) {
			active = true
			startTime = Date.now()
			scriptPath = path
			code = scriptCode
			language = lang
			scriptArgs = JSON.parse(JSON.stringify(args))
			recordedJob = undefined
		},
		recordInitialJob(_id: string, job: Job) {
			if (!active) return
			recordedJob = {
				initial_job: $state.snapshot(job) as Job,
				events: []
			}
		},
		recordEvent(_id: string, data: Record<string, any>) {
			if (!active) return
			if (!recordedJob) {
				recordedJob = {
					initial_job: (data as any).job
						? ($state.snapshot((data as any).job) as Job)
						: ({} as Job),
					events: []
				}
			}
			recordedJob.events.push({
				t: Date.now() - startTime,
				data: $state.snapshot(data) as Record<string, any>
			})
		},
		stop(): ScriptRecording {
			active = false
			const recording: ScriptRecording = {
				version: 1,
				type: 'script',
				recorded_at: new Date().toISOString(),
				script_path: scriptPath,
				total_duration_ms: Date.now() - startTime,
				code,
				language,
				args: scriptArgs,
				job: recordedJob ?? { initial_job: {} as Job, events: [] }
			}
			return recording
		},
		/** Convert to ActiveReplayData shape for JobLoader replay */
		toReplayData(recording: ScriptRecording): ActiveReplayData {
			// Find the job ID from the recorded initial_job
			const id = recording.job.initial_job?.id
			if (!id) {
				return { jobs: {} }
			}
			return {
				jobs: { [id]: recording.job }
			}
		},
		download(recording: ScriptRecording) {
			const blob = new Blob([JSON.stringify(recording, null, 2)], { type: 'application/json' })
			const url = URL.createObjectURL(blob)
			const a = document.createElement('a')
			a.href = url
			a.download = `script-recording-${(recording.script_path || 'untitled').replace(/\//g, '-')}-${Date.now()}.json`
			a.click()
			URL.revokeObjectURL(url)
		}
	}
}

export type ScriptRecordingStore = ActiveRecording & {
	readonly active: boolean
	start(path: string, code: string, lang: string, args: Record<string, any>): void
	stop(): ScriptRecording
	toReplayData(recording: ScriptRecording): ActiveReplayData
	download(recording: ScriptRecording): void
}
