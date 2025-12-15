import { describe, expect, it } from "vitest";
import { makeCreateTableQuery } from "./createTable";

function normalize(sql: string) {
	return sql.replace(/\s+/g, " ").trim();
}

describe("makeCreateTableQuery", () => {
	it("creates a basic table", () => {
		const sql = makeCreateTableQuery(
			{
				name: "users",
				columns: [
					{ name: "id", datatype: "INTEGER", primaryKey: true },
					{ name: "name", datatype: "VARCHAR", nullable: true },
				],
				foreignKeys: [],
			},
			"postgresql",
		);

		expect(normalize(sql)).toBe(
			normalize(`
				CREATE TABLE users (
					id INTEGER NOT NULL PRIMARY KEY,
					name VARCHAR
				);
			`),
		);
	});

	it("adds NOT NULL by default", () => {
		const sql = makeCreateTableQuery(
			{
				name: "t",
				columns: [
					{ name: "a", datatype: "INTEGER" },
					{ name: "b", datatype: "INTEGER", nullable: true },
				],
				foreignKeys: [],
			},
			"postgresql",
		);

		expect(normalize(sql)).toContain(
			normalize("a INTEGER NOT NULL"),
		);
		expect(normalize(sql)).toContain(
			normalize("b INTEGER"),
		);
	});

	it("inlines a single primary key", () => {
		const sql = makeCreateTableQuery(
			{
				name: "t",
				columns: [
					{ name: "id", datatype: "INTEGER", primaryKey: true },
					{ name: "code", datatype: "TEXT" },
				],
				foreignKeys: [],
			},
			"postgresql",
		);

		expect(sql).toContain("id INTEGER NOT NULL PRIMARY KEY");
		expect(sql).not.toContain("PRIMARY KEY (");
	});

	it("creates a composite primary key", () => {
		const sql = makeCreateTableQuery(
			{
				name: "t",
				columns: [
					{ name: "a", datatype: "INTEGER", primaryKey: true },
					{ name: "b", datatype: "INTEGER", primaryKey: true },
				],
				foreignKeys: [],
			},
			"postgresql",
		);

		expect(normalize(sql)).toContain(
			normalize("PRIMARY KEY (a, b)"),
		);
	});

	it("casts default values in PostgreSQL", () => {
		const sql = makeCreateTableQuery(
			{
				name: "t",
				columns: [
					{
						name: "created_at",
						datatype: "TIMESTAMP",
						defaultValue: "2024-01-01",
					},
				],
				foreignKeys: [],
			},
			"postgresql",
		);

		expect(sql).toContain(
			"DEFAULT CAST('2024-01-01' AS TIMESTAMP)",
		);
	});

	it("supports raw default expressions", () => {
		const sql = makeCreateTableQuery(
			{
				name: "t",
				columns: [
					{
						name: "created_at",
						datatype: "TIMESTAMP",
						defaultValue: "{now()}",
					},
				],
				foreignKeys: [],
			},
			"postgresql",
		);

		expect(sql).toContain("DEFAULT now()");
	});

	it("creates foreign keys with actions", () => {
		const sql = makeCreateTableQuery(
			{
				name: "posts",
				columns: [
					{ name: "id", datatype: "INTEGER", primaryKey: true },
					{ name: "author_id", datatype: "INTEGER" },
				],
				foreignKeys: [
					{
						targetTable: "users",
						columns: [
							{ sourceColumn: "author_id", targetColumn: "id" },
						],
						onDelete: "CASCADE",
						onUpdate: "NO ACTION",
					},
				],
			},
			"postgresql",
		);

		expect(normalize(sql)).toContain(
			normalize(`
			FOREIGN KEY (author_id)
			REFERENCES users (id)
			ON DELETE CASCADE
		`),
		);
	});

	it("uses schema when supported", () => {
		const sql = makeCreateTableQuery(
			{
				name: "users",
				columns: [{ name: "id", datatype: "INTEGER", primaryKey: true }],
				foreignKeys: [],
			},
			"postgresql",
			"public",
		);

		expect(sql.startsWith("CREATE TABLE public.users")).toBe(true);
	});

	it("uses quoted defaults in MySQL", () => {
		const sql = makeCreateTableQuery(
			{
				name: "t",
				columns: [
					{
						name: "name",
						datatype: "VARCHAR",
						defaultValue: "John",
					},
				],
				foreignKeys: [],
			},
			"mysql",
		);

		expect(sql).toContain("DEFAULT 'John'");
	});
});
