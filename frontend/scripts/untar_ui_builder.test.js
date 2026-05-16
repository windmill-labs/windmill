import test from 'node:test'
import assert from 'node:assert/strict'

import {
	computeSha256,
	getTarballUrl,
	shouldSkipPostinstall,
	verifyArtifactIntegrity
} from './untar_ui_builder.js'

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
		getTarballUrl({
			baseUrl: 'https://artifacts.example.com',
			version: '1234567',
			sha256: 'ignored'
		}),
		'https://artifacts.example.com/ui_builder-1234567.tar.gz'
	)
})

test('computeSha256 matches the expected digest', async () => {
	const digest = await computeSha256(Buffer.from('windmill-ui-builder'))
	assert.equal(digest, 'c747f031910fa8af923cda9b52204531666fe1c9b7c2213888dacb38b812175f')
})

test('verifyArtifactIntegrity throws on checksum mismatch', async () => {
	await assert.rejects(
		() =>
			verifyArtifactIntegrity(
				Buffer.from('artifact-bytes'),
				{
					baseUrl: 'https://artifacts.example.com',
					version: '1234567',
					sha256: 'deadbeef'
				}
			),
		/checksum mismatch/
	)
})
