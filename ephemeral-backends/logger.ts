import {
  appendFileSync,
  mkdirSync,
  unlinkSync,
  readdirSync,
  statSync,
} from "fs";
import path from "path";

const LOG_DIR = "/tmp/windmill-ephemeral-logs";
const LOG_RETENTION_MS = 24 * 60 * 60 * 1000; // 24 hours

export class Logger {
  private logFilePath: string;
  private prefix: string;

  constructor(identifier: string, prefix: string = "") {
    // Sanitize identifier to prevent path traversal
    const sanitizedId = identifier.replace(/[^a-zA-Z0-9-_]/g, "_");

    // Ensure log directory exists
    try {
      mkdirSync(LOG_DIR, { recursive: true });
    } catch (error) {
      // Directory might already exist
    }

    this.logFilePath = path.join(LOG_DIR, `${sanitizedId}.log`);
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
    const formatted = this.formatMessage("stdin ", message);
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
    const formatted = this.formatMessage("stderr", message);
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
    const formatted = this.formatMessage("warn  ", message);
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
    return path.join(LOG_DIR, `${sanitizedHash}.log`);
  }

  /**
   * Clean up old log files (older than 24 hours)
   */
  static cleanupOldLogs(): void {
    try {
      const files = readdirSync(LOG_DIR);
      const now = Date.now();
      let deletedCount = 0;

      for (const file of files) {
        if (!file.endsWith(".log")) continue;

        const filePath = path.join(LOG_DIR, file);
        try {
          const stats = statSync(filePath);
          const ageMs = now - stats.mtimeMs;

          if (ageMs > LOG_RETENTION_MS) {
            unlinkSync(filePath);
            deletedCount++;
            console.log(
              `Deleted old log file: ${file} (age: ${Math.floor(
                ageMs / 1000 / 60 / 60
              )}h)`
            );
          }
        } catch (error) {
          // Skip files we can't stat or delete
          console.error(`Failed to process log file ${file}:`, error);
        }
      }

      if (deletedCount > 0) {
        console.log(
          `Cleanup complete: deleted ${deletedCount} old log file(s)`
        );
      }
    } catch (error) {
      console.error("Failed to cleanup old logs:", error);
    }
  }
}
