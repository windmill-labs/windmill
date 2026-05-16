import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

import artifact from './ui_builder_artifact.json' with { type: 'json' }

export function getPostinstallSkipReason(
	cwd = process.cwd(),
	initCwd = process.env.INIT_CWD ?? ''
) {
	if (cwd.includes('node_modules')) {
		return 'running as dependency'
	}

	if (initCwd && initCwd !== cwd) {
		return 'not root project'
	}

	return null
}

export function shouldSkipPostinstall(cwd = process.cwd(), initCwd = process.env.INIT_CWD ?? '') {
	return getPostinstallSkipReason(cwd, initCwd) !== null
}

export function getTarballName(selectedArtifact = artifact) {
	return `ui_builder-${selectedArtifact.version}.tar.gz`
}

export function getTarballUrl(selectedArtifact = artifact) {
	return `${selectedArtifact.baseUrl}/${getTarballName(selectedArtifact)}`
}

export function computeSha256(buffer) {
	return createHash('sha256').update(buffer).digest('hex')
}

export function verifyArtifactIntegrity(buffer, selectedArtifact = artifact) {
	const actualSha256 = computeSha256(buffer)
	if (actualSha256 !== selectedArtifact.sha256) {
		throw new Error(
			`UI builder artifact checksum mismatch: expected ${selectedArtifact.sha256}, got ${actualSha256}`
		)
	}
}

export async function downloadArtifact(fetchImpl = fetch, selectedArtifact = artifact) {
	const tarballUrl = getTarballUrl(selectedArtifact)
	const response = await fetchImpl(tarballUrl)

	if (!response.ok) {
		throw new Error(
			`Failed to download UI builder artifact from ${tarballUrl}: ${response.status} ${response.statusText}`
		)
	}

	const buffer = Buffer.from(await response.arrayBuffer())
	verifyArtifactIntegrity(buffer, selectedArtifact)
	return buffer
}

async function extractArtifact(buffer, cwd = process.cwd()) {
	const { x } = await import('tar')
	const outputTarPath = path.join(cwd, 'ui_builder.tar.gz')
	const extractTo = path.join(cwd, 'static/ui_builder/')

	await fs.promises.mkdir(extractTo, { recursive: true })
	await fs.promises.writeFile(outputTarPath, buffer)

	try {
		await x({
			file: outputTarPath,
			cwd: extractTo,
			sync: false,
			gzip: true,
			preservePaths: false
		})
	} finally {
		await fs.promises.rm(outputTarPath, { force: true })
	}
}

export async function installUiBuilder(
	fetchImpl = fetch,
	cwd = process.cwd(),
	selectedArtifact = artifact
) {
	const buffer = await downloadArtifact(fetchImpl, selectedArtifact)
	await extractArtifact(buffer, cwd)
}

async function main() {
	const skipReason = getPostinstallSkipReason()
	if (skipReason) {
		console.log(`Skipping postinstall - ${skipReason}`)
		return
	}

	console.log('Running postinstall for root project')
	await installUiBuilder()
}

const entrypoint = process.argv[1] ? path.resolve(process.argv[1]) : null
const isMainModule = entrypoint === fileURLToPath(import.meta.url)

if (isMainModule) {
	await main()
}
