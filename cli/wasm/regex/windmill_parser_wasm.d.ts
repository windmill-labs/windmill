/* tslint:disable */
/* eslint-disable */
/**
* @param {string} code
* @returns {string}
*/
export function parse_bash(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_powershell(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_sql(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_mysql(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_oracledb(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_bigquery(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_snowflake(code: string): string;
/**
* @param {string} code
* @returns {string}
*/
export function parse_mssql(code: string): string;
/**
* @param {string} code
* @returns {string | undefined}
*/
export function parse_db_resource(code: string): string | undefined;
/**
* @param {string} code
* @returns {string}
*/
export function parse_graphql(code: string): string;
