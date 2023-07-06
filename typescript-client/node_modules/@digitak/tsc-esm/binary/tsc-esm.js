#!/usr/bin/env node
import { build } from "../library/index.js"
try {
	build()
} catch (error) {
	console.error(error)
	process.exit(1)
}
