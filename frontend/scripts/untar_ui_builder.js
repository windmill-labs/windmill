import path from 'path'
import fs from 'fs'

import { x } from 'tar'

const tarUrl = 'https://pub-06154ed168a24e73a86ab84db6bf15d8.r2.dev/ui_builder-d44b577.tar.gz'
const outputTarPath = path.join(process.cwd(), 'ui_builder.tar.gz')
const extractTo = path.join(process.cwd(), 'static/ui_builder/')

import { fileURLToPath } from 'url'
import { dirname } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// Download the tar file
const response = await fetch(tarUrl)
const buffer = await response.arrayBuffer()
await fs.promises.writeFile(outputTarPath, Buffer.from(buffer))

// Check if this script is being run from the package root
const isRootInstall = process.cwd() + '/scripts' === __dirname

if (isRootInstall) {
	console.log('Running postinstall: direct install')
	// Your postinstall logic here
} else {
	console.log('Skipping postinstall: installed as dependency')
	process.exit(0)
}

// Create extract directory if it doesn't exist
try {
	await fs.promises.mkdir(extractTo, { recursive: true })
} catch (err) {
	if (err.code !== 'EEXIST') {
		throw err
	}
}

await x({
	file: outputTarPath,
	cwd: extractTo,
	sync: false,
	gzip: true
})

await fs.promises.unlink(outputTarPath)
