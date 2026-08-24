import { describe, expect, it } from 'vitest'

import { expectedPaths, expectedTables } from './probe'

/**
 * These two are inference over what the export happens to say, so they are the part of the
 * probe that can be wrong while everything still type-checks. The network reads around them
 * either answer or throw.
 */

describe('expectedTables', () => {
	// The shape `datatableSchemaSql.ts` emits, and what every published project carries.
	const REAL = `BEGIN;
CREATE SCHEMA IF NOT EXISTS "calendly";
CREATE TABLE IF NOT EXISTS "calendly"."config" (
  "id" SERIAL NOT NULL,
  "host_name" text NOT NULL DEFAULT ''::text
);
CREATE TABLE IF NOT EXISTS "calendly"."bookings" ( "id" SERIAL NOT NULL );
COMMIT;`

	it('reads every table a migration creates', () => {
		expect(expectedTables(REAL)).toEqual(['calendly.config', 'calendly.bookings'])
	})

	it('does not mistake the schema for a table', () => {
		expect(expectedTables(REAL)).not.toContain('calendly')
	})

	it('reads the form without IF NOT EXISTS', () => {
		expect(expectedTables('CREATE TABLE "bitly"."links" (id int)')).toEqual(['bitly.links'])
	})

	it('is case- and whitespace-insensitive the way SQL is', () => {
		expect(expectedTables('create   table\n  "a" . "b" (x int)')).toEqual(['a.b'])
	})

	it('reports each table once, however many times it is named', () => {
		const sql = 'CREATE TABLE "a"."b" (x int); CREATE TABLE IF NOT EXISTS "a"."b" (x int);'
		expect(expectedTables(sql)).toEqual(['a.b'])
	})

	/**
	 * The answer that keeps a caller honest. An unquoted or unqualified `CREATE TABLE` is
	 * something this cannot resolve — the schema would come from `search_path` at run time —
	 * so it reads as nothing expected, and the caller treats that as "cannot tell" rather than
	 * as "no tables, so the migration must have run".
	 */
	it('claims nothing about SQL it cannot resolve', () => {
		expect(expectedTables('CREATE TABLE links (id int)')).toEqual([])
		expect(expectedTables('CREATE TABLE bitly.links (id int)')).toEqual([])
		expect(expectedTables('')).toEqual([])
	})
})

describe('expectedPaths', () => {
	const EXPORT = {
		project: { slug: 'calendly', name: 'Calendly', summary: '', readme: null },
		scripts: [{ path: 'f/calendly/book_slot' }],
		flows: [],
		apps: [{ path: 'f/calendly/booking' }],
		resources: [{ path: 'f/calendly/smtp' }],
		triggers: [],
		migrations: []
	} as any

	it('lists what the import will write, across every kind', () => {
		expect(expectedPaths(EXPORT, 'calendly')).toEqual([
			'f/calendly/book_slot',
			'f/calendly/booking',
			'f/calendly/smtp'
		])
	})

	// `installProject` retargets into the chosen folder, so the paths to look for are the
	// retargeted ones — checking the export's own would look for stubs that never landed.
	it('follows the folder the import was pointed at', () => {
		expect(expectedPaths(EXPORT, 'elsewhere')).toEqual([
			'f/elsewhere/book_slot',
			'f/elsewhere/booking',
			'f/elsewhere/smtp'
		])
	})
})
