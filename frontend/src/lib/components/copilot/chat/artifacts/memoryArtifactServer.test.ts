// The API is held to the same cases by `versions_follow_the_shared_cases` (backend/tests).
import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { fingerprint } from './artifactServer'
import { MemoryArtifactServer } from './memoryArtifactServer'

type Step = Record<string, Record<string, unknown>> & { expect?: Record<string, unknown> }

const { cases } = JSON.parse(
	readFileSync(
		new URL('../../../../../../../backend/tests/ai_artifacts_cases.json', import.meta.url),
		'utf8'
	)
) as { cases: { name: string; steps: Step[] }[] }

const versionHash = (v: unknown) =>
	Array.isArray(v) ? fingerprint(v[0], v[1]) : fingerprint('Plan', String(v))

describe('the in-memory server copy', () => {
	it.each(cases)('$name', async ({ steps }) => {
		const server = new MemoryArtifactServer()
		const ws = 'test-workspace'
		const id = 'a1'
		for (const [s, step] of steps.entries()) {
			const [op, args] = Object.entries(step).find(([key]) => key !== 'expect')!
			const name = (args.name as string | undefined) ?? 'Plan'
			const base = args.base === undefined ? '' : await versionHash(args.base)
			const version = args.version as number
			const content = args.content as string
			const answer: any =
				op === 'publish'
					? await server.publish(ws, { artifact_id: id, name, kind: 'md', version, content })
					: op === 'write'
						? await server.write(ws, id, { base_hash: base, version, name, content })
						: op === 'show'
							? await server.show(ws, id, {
									base_hash: base,
									pinned: args.pinned as boolean,
									version,
									name,
									content
								})
							: op === 'get'
								? await server.get(ws, id)
								: await server[op as 'trash' | 'restore'](ws, id)
			for (const [key, want] of Object.entries(step.expect ?? {})) {
				const got = answer?.[key] ?? answer?.artifact?.[key]
				const expected = key === 'latest_hash' ? await versionHash(want) : want
				expect(got, `step ${s}, ${key}`).toEqual(expected)
			}
		}
	})
})
