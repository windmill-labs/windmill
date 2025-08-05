/**
 * @fileoverview Main entry point for windmill-utils-internal package
 * 
 * This module provides utilities for handling Windmill flows, scripts, and schemas:
 * - Inline script extraction and replacement
 * - Path utilities for different programming languages  
 * - Schema parsing and conversion utilities
 * - Cross-platform path constants
 */

export * from "./inline-scripts";
export * from "./path-utils";
export * from "./parse";
export * from "./config";
export { SEP, DELIMITER } from "./constants";