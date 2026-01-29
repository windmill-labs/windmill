import { appendFileSync, mkdirSync } from "fs";
import path from "path";

export class Logger {
  private logFilePath: string;
  private prefix: string;

  constructor(identifier: string, prefix: string = "") {
    // Sanitize identifier to prevent path traversal
    const sanitizedId = identifier.replace(/[^a-zA-Z0-9-_]/g, "_");
    const logDir = "/tmp/windmill-ephemeral-logs";

    // Ensure log directory exists
    try {
      mkdirSync(logDir, { recursive: true });
    } catch (error) {
      // Directory might already exist
    }

    this.logFilePath = path.join(logDir, `${sanitizedId}.log`);
    this.prefix = prefix;

    // Write initial log entry
    const startTime = new Date().toISOString();
    this.log(`=== Log started at ${startTime} ===`);
  }

  private formatMessage(level: string, message: string): string {
    const timestamp = new Date().toISOString();
    const prefixStr = this.prefix ? `[${this.prefix}] ` : "";
    return `${timestamp} ${level} ${prefixStr}${message}\n`;
  }

  log(message: string): void {
    const formatted = this.formatMessage("INFO", message);
    try {
      appendFileSync(this.logFilePath, formatted);
    } catch (error) {
      // Fallback to console if file write fails
      console.error("Failed to write to log file:", error);
      console.log(formatted);
    }
    // Also log to stdout for systemd journal
    process.stdout.write(formatted);
  }

  error(message: string): void {
    const formatted = this.formatMessage("ERROR", message);
    try {
      appendFileSync(this.logFilePath, formatted);
    } catch (error) {
      console.error("Failed to write to log file:", error);
      console.error(formatted);
    }
    // Also log to stderr for systemd journal
    process.stderr.write(formatted);
  }

  warn(message: string): void {
    const formatted = this.formatMessage("WARN", message);
    try {
      appendFileSync(this.logFilePath, formatted);
    } catch (error) {
      console.error("Failed to write to log file:", error);
      console.warn(formatted);
    }
    process.stdout.write(formatted);
  }

  getLogFilePath(): string {
    return this.logFilePath;
  }

  static getLogFilePathForCommit(commitHash: string): string {
    // Sanitize commit hash to prevent path traversal
    const sanitizedHash = commitHash.replace(/[^a-f0-9]/g, "");
    if (sanitizedHash.length < 7 || sanitizedHash.length > 40) {
      throw new Error("Invalid commit hash");
    }
    return path.join("/tmp/windmill-ephemeral-logs", `${sanitizedHash}.log`);
  }
}
