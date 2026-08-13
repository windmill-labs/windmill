import { describe, it, expect } from 'vitest'
import { bashRunsInCustomImage } from './script_helpers'

// bashRunsInCustomImage decides whether the +Variable/+Resource pickers insert a
// curl/wget snippet (custom image, no wmill CLI) or the wmill CLI snippet. It must
// stay in sync with the worker's BashAnnotations grammar
// (backend/windmill-common/src/worker.rs): leading comment lines only, `# sandbox
// <image>` or bare `# docker` select a container; a bare `# sandbox` does not.

describe('bashRunsInCustomImage', () => {
	it('true for `# sandbox <image>` (spaced and compact)', () => {
		expect(bashRunsInCustomImage('# sandbox alpine:latest\necho hi')).toBe(true)
		expect(bashRunsInCustomImage('#sandbox   python:3.12-slim\n')).toBe(true)
	})

	it('true for a bare `# docker` annotation', () => {
		expect(bashRunsInCustomImage('# docker\necho hi')).toBe(true)
	})

	it('true when the sandbox line follows other leading comments (default template)', () => {
		expect(bashRunsInCustomImage('# shellcheck shell=bash\n# sandbox alpine:latest\necho hi')).toBe(
			true
		)
	})

	it('false for a bare `# sandbox` (nsjail-bash on the worker, wmill available)', () => {
		expect(bashRunsInCustomImage('# sandbox\necho hi')).toBe(false)
	})

	it('false for prose comments that merely contain the words', () => {
		expect(bashRunsInCustomImage('# sandboxed run below\necho hi')).toBe(false)
		expect(bashRunsInCustomImage('# runs in a docker container\necho hi')).toBe(false)
	})

	it('false when the annotation is not on a leading comment line', () => {
		expect(bashRunsInCustomImage('echo hi\n# sandbox alpine')).toBe(false)
		expect(bashRunsInCustomImage('msg="$1" # docker')).toBe(false)
	})

	it('false for a plain script with no annotations', () => {
		expect(bashRunsInCustomImage('# shellcheck shell=bash\necho hi')).toBe(false)
	})
})
