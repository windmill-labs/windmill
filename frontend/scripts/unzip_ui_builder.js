import fs from 'fs'
import https from 'https'
import unzipper from 'unzipper'
import path from 'path'

const zipUrl = 'https://pub-06154ed168a24e73a86ab84db6bf15d8.r2.dev/ui_builder-9db704a.zip'
const outputZipPath = path.join(process.cwd(), 'tmp.zip')
const extractTo = path.join(process.cwd(), 'static/ui_builder')

import { fileURLToPath } from 'url'
import { dirname, join } from 'path'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

// Check if this script is being run from the package root
const isRootInstall = process.cwd() + '/scripts' === __dirname

if (isRootInstall) {
	console.log('Running postinstall: direct install')
	// Your postinstall logic here
} else {
	console.log('Skipping postinstall: installed as dependency')
	process.exit(0)
}

// Download zip
https
	.get(zipUrl, (res) => {
		if (res.statusCode !== 200) {
			console.error(`Failed to download zip. Status code: ${res.statusCode}`)
			return
		}

		const fileStream = fs.createWriteStream(outputZipPath)
		res.pipe(fileStream)

		fileStream.on('finish', () => {
			fileStream.close(() => {
				console.log('Download complete. Extracting...')

				fs.createReadStream(outputZipPath)
					.pipe(unzipper.Extract({ path: extractTo }))
					.on('close', () => {
						console.log(`Extraction complete to ${extractTo}`)
						fs.unlinkSync(outputZipPath) // optional cleanup
					})
					.on('error', (err) => {
						console.error('Error during extraction:', err)
					})
			})
		})
	})
	.on('error', (err) => {
		console.error('Download error:', err)
	})
