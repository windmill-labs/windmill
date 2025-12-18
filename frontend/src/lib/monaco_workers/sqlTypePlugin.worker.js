/**
 * Custom TypeScript Language Service Plugin for SQL Type Inference
 *
 * This worker extends Monaco's TypeScriptWorker to inject type parameters
 * into SQL tagged template literals, enabling proper type checking.
 *
 * Example transformation:
 *   User writes: let x = sql`SELECT name FROM user`
 *   TS sees:     let x = sql<{ sql: "SELECT name FROM user" }>`SELECT name FROM user`
 */

import {
	TypeScriptWorker,
	ts,
	initialize
} from '@codingame/monaco-vscode-standalone-typescript-language-features/worker'

export function injectSqlTypes(code, queries) {
	let transformed = code
	let addedOffset = 0
	let offsetMap = {}
	queries = queries.filter((query) => query?.prepared?.columns)
	for (const query of queries) {
		let splitIdx = code?.indexOf('`', query.span[0] - 1)
		if (splitIdx === -1 || !splitIdx) continue
		let leftPart = transformed?.substring(0, splitIdx + addedOffset)
		let middlePart =
			'<{ ' +
			Object.entries(query?.prepared?.columns ?? {})
				.map(([key, type]) => `"${key}": ${type}`)
				.join('; ') +
			' }>'
		let rightPart = transformed?.substring(splitIdx + addedOffset)

		// Store the ORIGINAL position (splitIdx - 1), not the transformed one
		offsetMap[splitIdx - 1] = middlePart.length
		addedOffset += middlePart.length
		transformed = leftPart + middlePart + rightPart
	}
	return { transformed, offsetMap }
}

// Extend the TypeScriptWorker class
class SqlAwareTypeScriptWorker extends TypeScriptWorker {
	constructor(ctx, createData) {
		super(ctx, createData)

		// Map of file URI -> SQL query details
		this._sqlQueriesByFile = new Map()
		// Map of file URI -> version number (incremented when SQL queries change)
		this._fileVersions = new Map()
		// Cache of transformed code and offset maps per file version
		// Structure: fileUri -> {version, originalText, transformed, offsetMap, offsetMapEntries}
		this._transformedCodeCache = new Map()
	}

	/**
	 * Gets or computes the cached transformation result for a file
	 * This centralizes all transformation logic and caching
	 * @param {string} fileName - File name
	 * @returns {Object|null} Cached result with {version, originalText, transformed, offsetMap, offsetMapEntries}
	 */
	_getTransformResult(fileName) {
		const currentVersion = this.getScriptVersion(fileName)
		const cached = this._transformedCodeCache.get(fileName)

		// Return cached result if version matches
		if (cached && cached.version === currentVersion) {
			return cached
		}

		// Get original snapshot
		const originalSnapshot = super.getScriptSnapshot(fileName)
		if (!originalSnapshot) {
			return null
		}

		const queries = this._sqlQueriesByFile.get(fileName)
		if (!queries || queries.length === 0) {
			return null
		}

		// Compute transformation only once per version
		try {
			const originalText = originalSnapshot.getText(0, originalSnapshot.getLength())
			const { transformed, offsetMap } = injectSqlTypes(originalText, queries)

			// Pre-compute sorted offset map entries for fast position mapping
			const offsetMapEntries = Object.entries(offsetMap).sort(
				(a, b) => Number(a[0]) - Number(b[0])
			)

			const cacheEntry = {
				version: currentVersion,
				originalText,
				transformed,
				offsetMap,
				offsetMapEntries
			}

			this._transformedCodeCache.set(fileName, cacheEntry)
			return cacheEntry
		} catch (error) {
			console.error('[SqlTypePlugin] Error transforming source:', error)
			return null
		}
	}

	/**
	 * Override getScriptSnapshot to provide transformed source code with type annotations
	 * This is called by TypeScript when it needs to read source files
	 */
	getScriptSnapshot(fileName) {
		const cached = this._getTransformResult(fileName)

		if (!cached) {
			return super.getScriptSnapshot(fileName)
		}

		return ts.typescript.ScriptSnapshot.fromString(cached.transformed)
	}
	/**
	 * Maps a position from original code to transformed code
	 * @param {number} position - Position in original code
	 * @param {string} fileName - File name
	 * @returns {number} Position in transformed code
	 */
	_mapPositionToTransformed(position, fileName) {
		const cached = this._getTransformResult(fileName)
		if (!cached) {
			return position
		}

		let cumulativeOffset = 0
		for (const [pos, offset] of cached.offsetMapEntries) {
			const originalPos = Number(pos)
			// If the position is after this injection point in the original code
			if (position > originalPos) {
				cumulativeOffset += offset
			} else {
				break
			}
		}

		return position + cumulativeOffset
	}

	/**
	 * Maps a position from transformed code back to original code
	 * @param {number} position - Position in transformed code
	 * @param {string} fileName - File name
	 * @returns {number} Position in original code
	 */
	_mapPositionToOriginal(position, fileName) {
		const cached = this._getTransformResult(fileName)
		if (!cached) {
			return position
		}

		let cumulativeOffset = 0
		for (const [pos, offset] of cached.offsetMapEntries) {
			const originalPos = Number(pos)
			const transformedPos = originalPos + cumulativeOffset

			// If position in transformed code is after this injection point
			if (position > transformedPos) {
				cumulativeOffset += offset
			} else {
				break
			}
		}

		return position - cumulativeOffset
	}

	/**
	 * Override getQuickInfoAtPosition to map hover positions correctly
	 * This fixes the offset issue when hovering over code
	 */
	async getQuickInfoAtPosition(fileName, position) {
		// Map the position from original code to transformed code
		const transformedPosition = this._mapPositionToTransformed(position, fileName)

		// Get quick info from the base class using the transformed position
		const quickInfo = await super.getQuickInfoAtPosition(fileName, transformedPosition)

		if (!quickInfo) {
			return quickInfo
		}

		// Map the text span back to original positions
		if (quickInfo.textSpan) {
			quickInfo.textSpan.start = this._mapPositionToOriginal(quickInfo.textSpan.start, fileName)
		}

		return quickInfo
	}

	/**
	 * Override getCompletionsAtPosition to map autocomplete positions correctly
	 * This fixes the offset issue when showing autocomplete suggestions
	 */
	async getCompletionsAtPosition(fileName, position, options) {
		// Map the position from original code to transformed code
		const transformedPosition = this._mapPositionToTransformed(position, fileName)

		// Get completions from the base class using the transformed position
		const completions = await super.getCompletionsAtPosition(fileName, transformedPosition, options)

		if (!completions) {
			return completions
		}

		// Map all completion entry replacement spans back to original positions
		if (completions.entries) {
			for (const entry of completions.entries) {
				if (entry.replacementSpan) {
					entry.replacementSpan.start = this._mapPositionToOriginal(
						entry.replacementSpan.start,
						fileName
					)
				}
			}
		}

		return completions
	}

	/**
	 * Override getCompletionEntryDetails to map positions in detailed completion info
	 */
	async getCompletionEntryDetails(
		fileName,
		position,
		entryName,
		formatOptions,
		source,
		preferences,
		data
	) {
		// Map the position from original code to transformed code
		const transformedPosition = this._mapPositionToTransformed(position, fileName)

		// Get details from the base class using the transformed position
		const details = await super.getCompletionEntryDetails(
			fileName,
			transformedPosition,
			entryName,
			formatOptions,
			source,
			preferences,
			data
		)

		if (!details) {
			return details
		}

		// Map any code actions back to original positions
		if (details.codeActions) {
			for (const action of details.codeActions) {
				if (action.changes) {
					for (const change of action.changes) {
						if (change.textChanges) {
							for (const textChange of change.textChanges) {
								if (textChange.span) {
									textChange.span.start = this._mapPositionToOriginal(
										textChange.span.start,
										fileName
									)
								}
							}
						}
					}
				}
			}
		}

		return details
	}

	/**
	 * Maps a DocumentSpan (or derived types) from transformed to original positions
	 * DocumentSpan is used by DefinitionInfo, ReferenceEntry, ImplementationLocation, etc.
	 */
	_mapDocumentSpan(span, fileName) {
		if (!span) return span

		if (span.textSpan) {
			span.textSpan.start = this._mapPositionToOriginal(span.textSpan.start, fileName)
		}
		if (span.contextSpan) {
			span.contextSpan.start = this._mapPositionToOriginal(span.contextSpan.start, fileName)
		}
		if (span.originalTextSpan) {
			span.originalTextSpan.start = this._mapPositionToOriginal(
				span.originalTextSpan.start,
				fileName
			)
		}
		if (span.originalContextSpan) {
			span.originalContextSpan.start = this._mapPositionToOriginal(
				span.originalContextSpan.start,
				fileName
			)
		}

		return span
	}

	/**
	 * Maps an array of DocumentSpans
	 */
	_mapDocumentSpans(spans, fileName) {
		if (!spans) return spans
		return spans.map((span) => this._mapDocumentSpan(span, fileName))
	}

	/**
	 * Override getDefinitionAtPosition - Go to Definition
	 */
	async getDefinitionAtPosition(fileName, position) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const definitions = await super.getDefinitionAtPosition(fileName, transformedPosition)

		return this._mapDocumentSpans(definitions, fileName)
	}

	/**
	 * Override getDefinitionAndBoundSpan - Go to Definition with bound span
	 */
	async getDefinitionAndBoundSpan(fileName, position) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const result = await super.getDefinitionAndBoundSpan(fileName, transformedPosition)

		if (!result) return result

		// Map definitions
		if (result.definitions) {
			result.definitions = this._mapDocumentSpans(result.definitions, fileName)
		}

		// Map text span
		if (result.textSpan) {
			result.textSpan.start = this._mapPositionToOriginal(result.textSpan.start, fileName)
		}

		return result
	}

	/**
	 * Override getTypeDefinitionAtPosition - Go to Type Definition
	 */
	async getTypeDefinitionAtPosition(fileName, position) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const definitions = await super.getTypeDefinitionAtPosition(fileName, transformedPosition)

		return this._mapDocumentSpans(definitions, fileName)
	}

	/**
	 * Override getImplementationAtPosition - Go to Implementation
	 */
	async getImplementationAtPosition(fileName, position) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const implementations = await super.getImplementationAtPosition(fileName, transformedPosition)

		return this._mapDocumentSpans(implementations, fileName)
	}

	/**
	 * Override getReferencesAtPosition - Find All References
	 */
	async getReferencesAtPosition(fileName, position) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const references = await super.getReferencesAtPosition(fileName, transformedPosition)

		return this._mapDocumentSpans(references, fileName)
	}

	/**
	 * Override findReferences - Find References (alternative API)
	 */
	async findReferences(fileName, position) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const referencedSymbols = await super.findReferences(fileName, transformedPosition)

		if (!referencedSymbols) return referencedSymbols

		// Map each ReferencedSymbol
		for (const symbol of referencedSymbols) {
			// Map definition
			if (symbol.definition) {
				this._mapDocumentSpan(symbol.definition, fileName)
			}

			// Map references
			if (symbol.references) {
				symbol.references = this._mapDocumentSpans(symbol.references, fileName)
			}
		}

		return referencedSymbols
	}

	/**
	 * Override getDocumentHighlights - Highlight occurrences
	 */
	async getDocumentHighlights(fileName, position, filesToSearch) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const highlights = await super.getDocumentHighlights(
			fileName,
			transformedPosition,
			filesToSearch
		)

		if (!highlights) return highlights

		// Map each DocumentHighlights
		for (const highlight of highlights) {
			if (highlight.highlightSpans) {
				for (const span of highlight.highlightSpans) {
					if (span.textSpan) {
						span.textSpan.start = this._mapPositionToOriginal(span.textSpan.start, fileName)
					}
					if (span.contextSpan) {
						span.contextSpan.start = this._mapPositionToOriginal(span.contextSpan.start, fileName)
					}
				}
			}
		}

		return highlights
	}

	/**
	 * Override getRenameInfo - Check if rename is possible
	 */
	async getRenameInfo(fileName, position, options) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const renameInfo = await super.getRenameInfo(fileName, transformedPosition, options)

		if (!renameInfo) return renameInfo

		// Map triggerSpan if present
		if (renameInfo.triggerSpan) {
			renameInfo.triggerSpan.start = this._mapPositionToOriginal(
				renameInfo.triggerSpan.start,
				fileName
			)
		}

		return renameInfo
	}

	/**
	 * Override findRenameLocations - Get all rename locations
	 */
	async findRenameLocations(
		fileName,
		position,
		findInStrings,
		findInComments,
		providePrefixAndSuffixTextForRename
	) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const locations = await super.findRenameLocations(
			fileName,
			transformedPosition,
			findInStrings,
			findInComments,
			providePrefixAndSuffixTextForRename
		)

		return this._mapDocumentSpans(locations, fileName)
	}

	/**
	 * Override getSignatureHelpItems - Parameter hints
	 */
	async getSignatureHelpItems(fileName, position, options) {
		const transformedPosition = this._mapPositionToTransformed(position, fileName)
		const signatureHelp = await super.getSignatureHelpItems(fileName, transformedPosition, options)

		if (!signatureHelp) return signatureHelp

		// Map applicableSpan
		if (signatureHelp.applicableSpan) {
			signatureHelp.applicableSpan.start = this._mapPositionToOriginal(
				signatureHelp.applicableSpan.start,
				fileName
			)
		}

		return signatureHelp
	}

	/**
	 * Maps TextChange objects (used in refactorings and code fixes)
	 */
	_mapTextChanges(textChanges, fileName) {
		if (!textChanges) return textChanges

		for (const change of textChanges) {
			if (change.span) {
				change.span.start = this._mapPositionToOriginal(change.span.start, fileName)
			}
		}

		return textChanges
	}

	/**
	 * Maps FileTextChanges (used in refactorings and code fixes)
	 */
	_mapFileTextChanges(fileTextChanges, fileName) {
		if (!fileTextChanges) return fileTextChanges

		for (const fileChange of fileTextChanges) {
			if (fileChange.textChanges) {
				this._mapTextChanges(fileChange.textChanges, fileName)
			}
		}

		return fileTextChanges
	}

	/**
	 * Override getApplicableRefactors - Get available refactorings
	 */
	async getApplicableRefactors(
		fileName,
		positionOrRange,
		preferences,
		triggerReason,
		kind,
		includeInteractiveActions
	) {
		// Map position or range to transformed
		let transformedPositionOrRange = positionOrRange
		if (typeof positionOrRange === 'number') {
			transformedPositionOrRange = this._mapPositionToTransformed(positionOrRange, fileName)
		} else if (positionOrRange && typeof positionOrRange === 'object') {
			// It's a TextRange { pos, end }
			transformedPositionOrRange = {
				pos: this._mapPositionToTransformed(positionOrRange.pos, fileName),
				end: this._mapPositionToTransformed(positionOrRange.end, fileName)
			}
		}

		const refactors = await super.getApplicableRefactors(
			fileName,
			transformedPositionOrRange,
			preferences,
			triggerReason,
			kind,
			includeInteractiveActions
		)

		// Note: ApplicableRefactorInfo doesn't contain positions, so no mapping needed
		return refactors
	}

	/**
	 * Override getEditsForRefactor - Get edits for a specific refactoring
	 */
	async getEditsForRefactor(
		fileName,
		formatOptions,
		positionOrRange,
		refactorName,
		actionName,
		preferences,
		interactiveRefactorArguments
	) {
		// Map position or range to transformed
		let transformedPositionOrRange = positionOrRange
		if (typeof positionOrRange === 'number') {
			transformedPositionOrRange = this._mapPositionToTransformed(positionOrRange, fileName)
		} else if (positionOrRange && typeof positionOrRange === 'object') {
			transformedPositionOrRange = {
				pos: this._mapPositionToTransformed(positionOrRange.pos, fileName),
				end: this._mapPositionToTransformed(positionOrRange.end, fileName)
			}
		}

		const refactorEditInfo = await super.getEditsForRefactor(
			fileName,
			formatOptions,
			transformedPositionOrRange,
			refactorName,
			actionName,
			preferences,
			interactiveRefactorArguments
		)

		if (!refactorEditInfo) return refactorEditInfo

		// Map edits
		if (refactorEditInfo.edits) {
			this._mapFileTextChanges(refactorEditInfo.edits, fileName)
		}

		// Map renameLocation if present
		if (refactorEditInfo.renameLocation) {
			refactorEditInfo.renameLocation = this._mapPositionToOriginal(
				refactorEditInfo.renameLocation,
				fileName
			)
		}

		return refactorEditInfo
	}

	/**
	 * Override getCodeFixesAtPosition - Get quick fixes for errors
	 */
	async getCodeFixesAtPosition(fileName, start, end, errorCodes, formatOptions, preferences) {
		const transformedStart = this._mapPositionToTransformed(start, fileName)
		const transformedEnd = this._mapPositionToTransformed(end, fileName)

		const codeFixes = await super.getCodeFixesAtPosition(
			fileName,
			transformedStart,
			transformedEnd,
			errorCodes,
			formatOptions,
			preferences
		)

		if (!codeFixes) return codeFixes

		// Map each CodeFixAction
		for (const fix of codeFixes) {
			if (fix.changes) {
				this._mapFileTextChanges(fix.changes, fileName)
			}
			if (fix.fixAllDescription) {
				// fixAllDescription doesn't contain positions
			}
		}

		return codeFixes
	}

	/**
	 * Maps diagnostics positions from transformed code back to original code
	 * @param {Array} diagnostics - TypeScript diagnostics
	 * @param {string} fileName - File name
	 * @returns {Array} Diagnostics with corrected positions
	 */
	_mapDiagnostics(diagnostics, fileName) {
		try {
			return diagnostics.map((diagnostic) => {
				if (!diagnostic?.start) return diagnostic
				diagnostic.start = this._mapPositionToOriginal(diagnostic.start, fileName)
				return diagnostic
			})
		} catch (error) {
			console.error('[SqlTypePlugin] Error mapping diagnostics:', error)
			return diagnostics
		}
	}

	/**
	 * Creates diagnostics from SQL query preparation errors
	 * @param {string} fileName - File name
	 * @returns {Array} SQL error diagnostics
	 */
	_createSqlErrorDiagnostics(fileName) {
		const queries = this._sqlQueriesByFile.get(fileName)
		if (!queries || queries.length === 0) {
			return []
		}

		const cached = this._getTransformResult(fileName)
		const originalCode = cached?.originalText ?? ''
		if (!originalCode) {
			// Fallback if no cached result
			const originalSnapshot = super.getScriptSnapshot(fileName)
			if (!originalSnapshot) {
				return []
			}
		}

		const sqlDiagnostics = []
		for (const query of queries) {
			let messageText = query?.prepared?.error
			if (typeof messageText === 'string') {
				let queryStartIdx = originalCode.indexOf('`', (query.span?.[0] || 1) - 1) + 1
				// Create a diagnostic error for this query
				let prefix = 'Failed to prepare query: db error: ERROR: '
				if (messageText.startsWith(prefix)) messageText = messageText.substring(prefix.length)
				const diagnostic = {
					code: 'SQL_PREPARATION_ERROR',
					category: ts.typescript.DiagnosticCategory.Error,
					messageText,
					file: fileName,
					start: queryStartIdx,
					length: query.span?.[1] ? query.span[1] - queryStartIdx - 2 : 0,
					source: 'sql'
				}
				sqlDiagnostics.push(diagnostic)
			}
		}

		return sqlDiagnostics
	}

	async getSyntacticDiagnostics(fileName) {
		const diagnostics = await super.getSyntacticDiagnostics(fileName)
		const mappedDiagnostics = this._mapDiagnostics(diagnostics, fileName)
		const sqlDiagnostics = this._createSqlErrorDiagnostics(fileName)
		return [...mappedDiagnostics, ...sqlDiagnostics]
	}

	async getSemanticDiagnostics(fileName) {
		const diagnostics = await super.getSemanticDiagnostics(fileName)
		const mappedDiagnostics = this._mapDiagnostics(diagnostics, fileName)
		const sqlDiagnostics = this._createSqlErrorDiagnostics(fileName)
		return [...mappedDiagnostics, ...sqlDiagnostics]
	}

	async getSuggestionDiagnostics(fileName) {
		const diagnostics = await super.getSuggestionDiagnostics(fileName)
		return this._mapDiagnostics(diagnostics, fileName)
	}

	async getCompilerOptionsDiagnostics(fileName) {
		const diagnostics = await super.getCompilerOptionsDiagnostics(fileName)
		return this._mapDiagnostics(diagnostics, fileName)
	}

	/**
	 * Override getScriptVersion to return an incremented version when SQL queries change
	 * This forces TypeScript to invalidate its cache and re-read the snapshot
	 */
	getScriptVersion(fileName) {
		const baseVersion = super.getScriptVersion(fileName)
		const sqlVersion = this._fileVersions.get(fileName) || 0
		// Combine base version with SQL version to create a unique version string
		return `${baseVersion}-sql${sqlVersion}`
	}

	/**
	 * Custom method to update SQL query information from the main thread
	 * This is called via worker messaging from the Editor
	 *
	 * @param {string} fileUri - URI of the file being edited
	 * @param {Array} queries - Array of SQL query details
	 */
	async updateSqlQueries(fileUri, queries) {
		// Invalidate cache when SQL queries change
		this._transformedCodeCache.delete(fileUri)

		if (!queries || queries.length === 0) {
			this._sqlQueriesByFile.delete(fileUri)
		} else {
			this._sqlQueriesByFile.set(fileUri, queries)
		}

		// Increment the version to force TypeScript to re-read the snapshot
		const currentVersion = this._fileVersions.get(fileUri) || 0
		this._fileVersions.set(fileUri, currentVersion + 1)

		return true
	}
}

// Create function that Monaco expects
export function create(ctx, createData) {
	return new SqlAwareTypeScriptWorker(ctx, createData)
}

if (typeof self !== 'undefined' && self) {
	// Initialize the worker
	self.onmessage = () => {
		initialize((ctx, createData) => {
			return create(ctx, createData)
		})
	}

	// This function is called by Monaco's TypeScript worker if customWorkerPath is used
	// It receives the base TypeScriptWorker class, TypeScript API (ts), and libFileMap
	// We'll keep this as a fallback
	// @ts-ignore
	self.customTSWorkerFactory = function (TypeScriptWorkerBase, tsApi, libFileMap) {
		return SqlAwareTypeScriptWorker
	}

	console.log('[SqlTypePlugin] Worker module loaded')
}
