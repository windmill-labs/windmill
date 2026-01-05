import { describe, expect, it } from 'vitest'
import { makeAlterTableQueries } from './alterTable'

function normalize(sql: string) {
	return sql.replace(/\s+/g, ' ').trim()
}

describe('makeAlterTableQueries', () => {
	it('adds a column', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'addColumn',
						column: {
							name: 'age',
							datatype: 'INTEGER',
							nullable: true
						}
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(1)
		expect(normalize(queries[0])).toBe(
			normalize(`
				ALTER TABLE users
				ADD COLUMN age INTEGER;
			`)
		)
	})

	it('drops a column', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'dropColumn',
						name: 'age'
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(normalize('ALTER TABLE users DROP COLUMN age;'))
	})

	it('renames a column', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'renameColumn',
						from: 'fullname',
						to: 'name'
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE users RENAME COLUMN fullname TO name;')
		)
	})

	it('alters column type and nullability', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'alterColumn',
						name: 'age',
						changes: {
							datatype: 'BIGINT',
							nullable: false
						}
					}
				]
			},
			'postgresql'
		)

		expect(queries).toHaveLength(2)

		expect(normalize(queries[0])).toBe(normalize('ALTER TABLE users ALTER COLUMN age TYPE BIGINT;'))

		expect(normalize(queries[1])).toBe(
			normalize('ALTER TABLE users ALTER COLUMN age SET NOT NULL;')
		)
	})

	it('sets and drops default values', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'alterColumn',
						name: 'created_at',
						changes: {
							defaultValue: '{now()}'
						}
					},
					{
						kind: 'alterColumn',
						name: 'created_at',
						changes: {
							defaultValue: undefined
						}
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toEqual(
			'ALTER TABLE users ALTER COLUMN created_at SET DEFAULT now();'
		)

		expect(normalize(queries[1])).toEqual('ALTER TABLE users ALTER COLUMN created_at DROP DEFAULT;')
	})

	it('adds a foreign key', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'posts',
				operations: [
					{
						kind: 'addForeignKey',
						foreignKey: {
							targetTable: 'users',
							columns: [
								{
									sourceColumn: 'user_id',
									targetColumn: 'id'
								}
							],
							onDelete: 'CASCADE',
							onUpdate: 'NO ACTION'
						}
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(
			normalize(`
				ALTER TABLE posts
				ADD CONSTRAINT fk_posts_user_id_users_id
				FOREIGN KEY (user_id)
				REFERENCES users (id)
				ON DELETE CASCADE;
			`)
		)
	})

	it('drops a foreign key using constraint in PostgreSQL', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'posts',
				operations: [
					{
						kind: 'dropForeignKey',
						name: 'fk_posts_user'
					}
				]
			},
			'postgresql'
		)

		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE posts DROP CONSTRAINT fk_posts_user;')
		)
	})

	it('drops a foreign key in MySQL', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'posts',
				operations: [
					{
						kind: 'dropForeignKey',
						name: 'fk_posts_user'
					}
				]
			},
			'mysql'
		)

		expect(normalize(queries[0])).toBe(
			normalize('ALTER TABLE posts DROP FOREIGN KEY fk_posts_user;')
		)
	})

	it('uses schema when provided', () => {
		const queries = makeAlterTableQueries(
			{
				name: 'users',
				operations: [
					{
						kind: 'addColumn',
						column: {
							name: 'age',
							datatype: 'INTEGER'
						}
					}
				]
			},
			'postgresql',
			'public'
		)

		expect(queries[0].startsWith('ALTER TABLE public.users')).toBe(true)
	})
})
