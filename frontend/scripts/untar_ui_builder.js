import path from 'path'
import fs from 'fs'


// Check if we're in node_modules (installed as dependency)
if (process.cwd().includes('node_modules')) {
	console.log('Skipping postinstall - running as dependency');
	process.exit(0);
}

// Check if we're in the root project
if (process.env.INIT_CWD && process.env.INIT_CWD !== process.cwd()) {
	console.log('Skipping postinstall - not root project');
	process.exit(0);
}

// Your actual postinstall logic here
console.log('Running postinstall for root project');


import { x } from 'tar'

const tarUrl = 'https://pub-06154ed168a24e73a86ab84db6bf15d8.r2.dev/ui_builder-0e5c66f.tar.gz'
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
