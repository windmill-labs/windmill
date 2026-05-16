import test from 'node:test'
import assert from 'node:assert/strict'

import {
	computeSha256,
	downloadArtifact,
	getPostinstallSkipReason,
	getTarballUrl,
	shouldSkipPostinstall,
	verifyArtifactIntegrity
} from './untar_ui_builder.js'

function fakeResponse({ ok = true, status = 200, statusText = 'OK', body = Buffer.alloc(0) } = {}) {
	return {
		ok,
		status,
		statusText,
		async arrayBuffer() {
			return body.buffer.slice(body.byteOffset, body.byteOffset + body.byteLength)
		}
	}
}

const sampleArtifact = {
	baseUrl: 'https://artifacts.example.com',
	version: '1234567',
	sha256: 'ignored'
}

test('getPostinstallSkipReason flags dependency installs inside node_modules', () => {
	assert.equal(
		getPostinstallSkipReason('/tmp/node_modules/windmill/frontend', ''),
		'running as dependency'
	)
})

test('getPostinstallSkipReason flags installs outside the root project', () => {
	assert.equal(
		getPostinstallSkipReason('/tmp/windmill/frontend', '/tmp/windmill'),
		'not root project'
	)
})

test('getPostinstallSkipReason returns null at the root project', () => {
	assert.equal(getPostinstallSkipReason('/tmp/windmill/frontend', '/tmp/windmill/frontend'), null)
})

test('shouldSkipPostinstall skips dependency installs inside node_modules', () => {
	assert.equal(shouldSkipPostinstall('/tmp/node_modules/windmill/frontend', ''), true)
})

test('shouldSkipPostinstall skips when INIT_CWD differs from the current project', () => {
	assert.equal(shouldSkipPostinstall('/tmp/windmill/frontend', '/tmp/windmill'), true)
})

test('shouldSkipPostinstall runs at the root project', () => {
	assert.equal(shouldSkipPostinstall('/tmp/windmill/frontend', '/tmp/windmill/frontend'), false)
})

test('getTarballUrl builds the pinned artifact URL', () => {
	assert.equal(
		getTarballUrl(sampleArtifact),
		'https://artifacts.example.com/ui_builder-1234567.tar.gz'
	)
})

test('computeSha256 matches the expected digest', () => {
	const digest = computeSha256(Buffer.from('windmill-ui-builder'))
	assert.equal(digest, 'c747f031910fa8af923cda9b52204531666fe1c9b7c2213888dacb38b812175f')
})

test('verifyArtifactIntegrity passes when the checksum matches', () => {
	const buffer = Buffer.from('artifact-bytes')
	assert.doesNotThrow(() =>
		verifyArtifactIntegrity(buffer, { ...sampleArtifact, sha256: computeSha256(buffer) })
	)
})

test('verifyArtifactIntegrity throws on checksum mismatch', () => {
	assert.throws(
		() =>
			verifyArtifactIntegrity(Buffer.from('artifact-bytes'), {
				...sampleArtifact,
				sha256: 'deadbeef'
			}),
		/checksum mismatch/
	)
})

test('downloadArtifact returns the buffer when the checksum matches', async () => {
	const body = Buffer.from('artifact-bytes')
	const fetchImpl = async () => fakeResponse({ body })
	const result = await downloadArtifact(fetchImpl, {
		...sampleArtifact,
		sha256: computeSha256(body)
	})
	assert.ok(result.equals(body))
})

test('downloadArtifact throws on a non-ok HTTP status', async () => {
	const fetchImpl = async () => fakeResponse({ ok: false, status: 404, statusText: 'Not Found' })
	await assert.rejects(() => downloadArtifact(fetchImpl, sampleArtifact), /404 Not Found/)
})

test('downloadArtifact throws when the downloaded bytes fail verification', async () => {
	const fetchImpl = async () => fakeResponse({ body: Buffer.from('tampered') })
	await assert.rejects(
		() => downloadArtifact(fetchImpl, { ...sampleArtifact, sha256: 'deadbeef' }),
		/checksum mismatch/
	)
})
