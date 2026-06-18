import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'

import artifact from './ui_builder_artifact.json' with { type: 'json' }

// Skip when installed as a dependency or outside the root project
if (process.cwd().includes('node_modules')) {
	console.log('Skipping postinstall - running as dependency')
	process.exit(0)
}
if (process.env.INIT_CWD && process.env.INIT_CWD !== process.cwd()) {
	console.log('Skipping postinstall - not root project')
	process.exit(0)
}

console.log('Running postinstall for root project')

const tarUrl = `${artifact.baseUrl}/ui_builder-${artifact.version}.tar.gz`
const response = await fetch(tarUrl)
if (!response.ok) {
	throw new Error(`Failed to download ${tarUrl}: ${response.status} ${response.statusText}`)
}

const buffer = Buffer.from(await response.arrayBuffer())
const sha256 = createHash('sha256').update(buffer).digest('hex')
if (sha256 !== artifact.sha256) {
	throw new Error(
		`UI builder artifact checksum mismatch: expected ${artifact.sha256}, got ${sha256}`
	)
}

const outputTarPath = path.join(process.cwd(), 'ui_builder.tar.gz')
const extractTo = path.join(process.cwd(), 'static/ui_builder/')

await fs.promises.mkdir(extractTo, { recursive: true })
await fs.promises.writeFile(outputTarPath, buffer)

const { x } = await import('tar')
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
