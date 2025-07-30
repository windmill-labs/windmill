import { JobService } from '$lib/gen'
import { tryEvery } from '$lib/utils'

export interface JobStatus {
	status: 'running' | 'success' | 'failure'
	result?: any
	error?: string
}

export interface JobOptions {
	onProgress?: (status: JobStatus) => void
	timeout?: number
	workspace: string
	interval?: number
	timeoutMessage?: string
}

interface JobEntry {
	controller: AbortController
	startTime: number
}

export class JobManager {
	private activeJobs = new Map<string, JobEntry>()
	private cleanupInterval: NodeJS.Timeout | null = null
	private readonly STALE_TIMEOUT = 300000 // 5 minutes
	private readonly CLEANUP_INTERVAL = 60000 // 1 minute

	constructor() {
		this.startCleanupTimer()
	}

	private startCleanupTimer() {
		if (typeof window !== 'undefined') {
			this.cleanupInterval = setInterval(() => {
				this.cleanup()
			}, this.CLEANUP_INTERVAL)
		}
	}

	private cleanup() {
		const now = Date.now()
		const staleJobs: string[] = []

		for (const [jobId, entry] of this.activeJobs.entries()) {
			if (now - entry.startTime > this.STALE_TIMEOUT) {
				entry.controller.abort()
				staleJobs.push(jobId)
			}
		}

		staleJobs.forEach(jobId => {
			this.activeJobs.delete(jobId)
		})

		if (staleJobs.length > 0) {
			console.warn(`Cleaned up ${staleJobs.length} stale job controllers`)
		}
	}

	async runWithProgress<T>(
		jobRunner: () => Promise<string>,
		options: JobOptions
	): Promise<T> {
		const {
			onProgress,
			timeout = 60000,
			workspace,
			interval = 500,
			timeoutMessage = `Job timed out after ${timeout / 1000}s`
		} = options

		const controller = new AbortController()
		const jobId = await jobRunner()

		this.activeJobs.set(jobId, {
			controller,
			startTime: Date.now()
		})

		try {
			onProgress?.({ status: 'running' })

			const result = await tryEvery({
				tryCode: async () => {
					if (controller.signal.aborted) {
						throw new Error('Job was cancelled')
					}

					const jobResult = await JobService.getCompletedJob({
						workspace,
						id: jobId
					})

					const success = !!jobResult.success
					const status: JobStatus = {
						status: success ? 'success' : 'failure',
						result: jobResult.result,
						error: success ? undefined : (jobResult.result as any)?.error?.message || 'Job failed'
					}

					onProgress?.(status)

					if (!success) {
						throw new Error(status.error)
					}

					return jobResult.result as T
				},
				timeoutCode: async () => {
					try {
						await JobService.cancelQueuedJob({
							workspace,
							id: jobId,
							requestBody: { reason: timeoutMessage }
						})
					} catch (err) {
						console.error('Failed to cancel job:', err)
					}

					onProgress?.({ status: 'failure', error: timeoutMessage })
					throw new Error(timeoutMessage)
				},
				interval,
				timeout
			})

			return result as T
		} finally {
			this.activeJobs.delete(jobId)
		}
	}

	cancel(jobId: string) {
		const entry = this.activeJobs.get(jobId)
		if (entry) {
			entry.controller.abort()
			this.activeJobs.delete(jobId)
		}
	}

	cancelAll() {
		this.activeJobs.forEach(entry => entry.controller.abort())
		this.activeJobs.clear()
	}

	isActive(jobId: string): boolean {
		return this.activeJobs.has(jobId)
	}

	get activeJobCount(): number {
		return this.activeJobs.size
	}

	destroy() {
		if (this.cleanupInterval) {
			clearInterval(this.cleanupInterval)
			this.cleanupInterval = null
		}
		this.cancelAll()
	}
}

// Singleton instance for global usage
export const jobManager = new JobManager()
