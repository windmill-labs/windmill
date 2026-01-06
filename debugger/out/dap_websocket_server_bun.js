#!/usr/bin/env bun
// @bun

// dap_websocket_server_bun.ts
var {spawn } = globalThis.Bun;
import { mkdtemp, writeFile, unlink, rmdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
var LOG_LEVEL = process.env.LOG_LEVEL || "DEBUG";
var logger = {
  debug: (...args) => {
    if (LOG_LEVEL === "DEBUG")
      console.log("[DEBUG]", new Date().toISOString(), ...args);
  },
  info: (...args) => console.log("[INFO]", new Date().toISOString(), ...args),
  warn: (...args) => console.warn("[WARN]", new Date().toISOString(), ...args),
  error: (...args) => console.error("[ERROR]", new Date().toISOString(), ...args)
};
function decodeVLQ(encoded) {
  const VLQ_BASE64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const values = [];
  let shift = 0;
  let value = 0;
  for (const char of encoded) {
    const digit = VLQ_BASE64.indexOf(char);
    if (digit === -1)
      continue;
    const hasContinuation = digit & 32;
    value += (digit & 31) << shift;
    if (hasContinuation) {
      shift += 5;
    } else {
      const isNegative = value & 1;
      value = value >> 1;
      values.push(isNegative ? -value : value);
      value = 0;
      shift = 0;
    }
  }
  return values;
}
function parseSourceMapLineMapping(sourceMapUrl) {
  const mappings = {
    originalToTranspiled: new Map,
    transpiledToOriginal: new Map
  };
  try {
    const match = sourceMapUrl.match(/^data:application\/json;base64,(.+)$/);
    if (!match) {
      logger.warn("Source map is not a base64 data URL");
      return mappings;
    }
    const decoded = atob(match[1]);
    const sourceMap = JSON.parse(decoded);
    if (!sourceMap.mappings) {
      return mappings;
    }
    const lines = sourceMap.mappings.split(";");
    let originalLine = 0;
    for (let transpiledLine = 0;transpiledLine < lines.length; transpiledLine++) {
      const segments = lines[transpiledLine].split(",");
      for (const segment of segments) {
        if (!segment)
          continue;
        const values = decodeVLQ(segment);
        if (values.length >= 3) {
          originalLine += values[2];
          if (!mappings.originalToTranspiled.has(originalLine)) {
            mappings.originalToTranspiled.set(originalLine, transpiledLine);
            logger.debug(`Source map: original line ${originalLine} -> transpiled line ${transpiledLine}`);
          }
          const existing = mappings.transpiledToOriginal.get(transpiledLine);
          if (existing === undefined || originalLine > existing) {
            mappings.transpiledToOriginal.set(transpiledLine, originalLine);
            logger.debug(`Source map: transpiled line ${transpiledLine} -> original line ${originalLine}`);
          }
        }
      }
    }
  } catch (error) {
    logger.error("Failed to parse source map:", error);
  }
  return mappings;
}

class DebugSession {
  ws;
  seq = 1;
  initialized = false;
  configured = false;
  running = false;
  terminatedSent = false;
  process = null;
  inspectorWs = null;
  inspectorSeq = 1;
  pendingInspectorRequests = new Map;
  scriptPath = null;
  tempDir = null;
  tempFile = null;
  breakpoints = new Map;
  breakpointIds = new Map;
  scripts = new Map;
  mainScriptId = null;
  sourceMapMappings = {
    originalToTranspiled: new Map,
    transpiledToOriginal: new Map
  };
  callFrames = [];
  variablesRefCounter = 1;
  scopesMap = new Map;
  objectsMap = new Map;
  callMain = false;
  mainArgs = {};
  isStepping = false;
  pendingConsoleOutput = [];
  scriptResult = undefined;
  pausedAtBreakpoint = false;
  envVars = {};
  nsjailConfig;
  bunPath = "bun";
  windmillPath;
  nodeModulesPath;
  constructor(ws, options) {
    this.ws = ws;
    this.nsjailConfig = options?.nsjailConfig;
    this.bunPath = options?.bunPath || "bun";
    this.windmillPath = options?.windmillPath;
  }
  nextSeq() {
    return this.seq++;
  }
  nextInspectorSeq() {
    return this.inspectorSeq++;
  }
  nextVarRef() {
    return this.variablesRefCounter++;
  }
  sendMessage(msg) {
    const data = JSON.stringify(msg);
    logger.debug("Sending DAP:", data);
    this.ws.send(data);
  }
  sendResponse(request, success = true, body = {}, message = "") {
    this.sendMessage({
      seq: this.nextSeq(),
      type: "response",
      command: request.command || "",
      request_seq: request.seq,
      success,
      message,
      body
    });
  }
  sendEvent(event, body = {}) {
    this.sendMessage({
      seq: this.nextSeq(),
      type: "event",
      event,
      body
    });
  }
  async sendInspectorCommand(method, params = {}) {
    if (!this.inspectorWs || this.inspectorWs.readyState !== WebSocket.OPEN) {
      throw new Error("Inspector not connected");
    }
    const id = this.nextInspectorSeq();
    const message = { id, method, params };
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingInspectorRequests.delete(id);
        reject(new Error(`Inspector command timeout: ${method}`));
      }, 1e4);
      this.pendingInspectorRequests.set(id, {
        resolve: (value) => {
          clearTimeout(timeout);
          resolve(value);
        },
        reject: (error) => {
          clearTimeout(timeout);
          reject(error);
        }
      });
      logger.debug("Sending to inspector:", JSON.stringify(message));
      this.inspectorWs.send(JSON.stringify(message));
    });
  }
  handleInspectorMessage(data) {
    try {
      const message = JSON.parse(data);
      logger.debug("Inspector message:", data);
      if (message.id !== undefined) {
        const pending = this.pendingInspectorRequests.get(message.id);
        if (pending) {
          this.pendingInspectorRequests.delete(message.id);
          if (message.error) {
            pending.reject(new Error(message.error.message));
          } else {
            pending.resolve(message);
          }
        }
        return;
      }
      if (message.method) {
        this.handleInspectorEvent(message);
      }
    } catch (error) {
      logger.error("Failed to parse inspector message:", error);
    }
  }
  handleInspectorEvent(message) {
    const params = message.params || {};
    switch (message.method) {
      case "Debugger.scriptParsed":
        this.handleScriptParsed(params);
        break;
      case "Debugger.paused":
        this.handlePaused(params);
        break;
      case "Debugger.resumed":
        this.sendEvent("continued", { threadId: 1 });
        break;
      case "Runtime.consoleAPICalled":
        this.handleConsoleOutput(params);
        break;
      case "Console.messageAdded":
        this.handleConsoleMessageAdded(params);
        break;
      case "Runtime.exceptionThrown":
        this.handleException(params);
        break;
      case "Debugger.breakpointResolved":
        logger.info(`Breakpoint resolved: ${JSON.stringify(params)}`);
        break;
      default:
        logger.debug("Unhandled inspector event:", message.method);
    }
  }
  handleScriptParsed(script) {
    this.scripts.set(script.scriptId, script);
    if (script.url && this.scriptPath && script.url.endsWith(this.scriptPath.split("/").pop())) {
      this.mainScriptId = script.scriptId;
      logger.info(`Main script parsed: ${script.url} (ID: ${script.scriptId})`);
      if (script.sourceMapURL) {
        logger.info("Parsing source map for line number mapping...");
        this.sourceMapMappings = parseSourceMapLineMapping(script.sourceMapURL);
        logger.info(`Source map parsed: ${this.sourceMapMappings.originalToTranspiled.size} original->transpiled, ${this.sourceMapMappings.transpiledToOriginal.size} transpiled->original`);
      }
    }
  }
  async applyBreakpointsByUrl() {
    logger.info(`applyBreakpointsByUrl called. scriptPath: ${this.scriptPath}, breakpoints: ${JSON.stringify([...this.breakpoints.entries()])}`);
    if (!this.scriptPath) {
      logger.warn("No scriptPath set, skipping breakpoints");
      return;
    }
    for (const [filePath, ids] of this.breakpointIds) {
      for (const id of ids) {
        try {
          await this.sendInspectorCommand("Debugger.removeBreakpoint", { breakpointId: id });
        } catch {}
      }
    }
    this.breakpointIds.clear();
    for (const [filePath, lines] of this.breakpoints) {
      logger.info(`Setting breakpoints for ${filePath}: lines ${lines}`);
      const ids = [];
      for (const line of lines) {
        try {
          const urlRegex = this.scriptPath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
          const originalLine0Indexed = line;
          let transpiledLine = originalLine0Indexed;
          if (this.sourceMapMappings.originalToTranspiled.size > 0) {
            const mappedLine = this.sourceMapMappings.originalToTranspiled.get(originalLine0Indexed);
            if (mappedLine !== undefined) {
              transpiledLine = mappedLine;
              logger.info(`Source map: original line ${originalLine0Indexed} -> transpiled line ${transpiledLine}`);
            } else {
              logger.warn(`Line ${originalLine0Indexed} not in source map, using as-is`);
            }
          }
          logger.info(`Setting breakpoint: user line ${line} -> original line ${originalLine0Indexed} (0-idx) -> transpiled line ${transpiledLine}`);
          const response = await this.sendInspectorCommand("Debugger.setBreakpointByUrl", {
            lineNumber: transpiledLine,
            urlRegex,
            columnNumber: 0
          });
          logger.info(`Breakpoint response: ${JSON.stringify(response)}`);
          if (response.result?.breakpointId) {
            ids.push(response.result.breakpointId);
            logger.info(`Breakpoint set at transpiled line ${transpiledLine} (user line ${line}, ID: ${response.result.breakpointId})`);
          } else {
            logger.warn(`No breakpointId in response for line ${line}`);
          }
        } catch (error) {
          logger.error(`Failed to set breakpoint at line ${line}:`, error);
        }
      }
      this.breakpointIds.set(filePath, ids);
    }
    logger.info(`Finished setting breakpoints. Total IDs: ${[...this.breakpointIds.values()].flat().length}`);
  }
  initialPauseDone = false;
  async handlePaused(params) {
    const callFrames = params.callFrames;
    const reason = params.reason;
    logger.info(`handlePaused called: reason=${reason}, lineNumber=${callFrames?.[0]?.location?.lineNumber}, scriptId=${callFrames?.[0]?.location?.scriptId}`);
    if (callFrames) {
      this.callFrames = callFrames;
    }
    const isInitialPause = !this.initialPauseDone && reason === "DebuggerStatement" && callFrames && callFrames.length > 0 && callFrames[0].location.lineNumber === 0;
    logger.info(`isInitialPause=${isInitialPause}, initialPauseDone=${this.initialPauseDone}`);
    if (isInitialPause) {
      this.initialPauseDone = true;
      logger.info("Initial pause from injected debugger statement");
      logger.info("Re-applying breakpoints after script is parsed...");
      await this.applyBreakpointsByUrl();
      try {
        await this.sendInspectorCommand("Debugger.resume", {});
        logger.info("Auto-resumed after initial pause");
      } catch (error) {
        logger.error("Failed to auto-resume:", error);
      }
      return;
    }
    this.initialPauseDone = true;
    let dapReason;
    switch (reason) {
      case "Breakpoint":
      case "breakpoint":
        dapReason = "breakpoint";
        break;
      case "DebuggerStatement":
        dapReason = "breakpoint";
        break;
      case "step":
        dapReason = "step";
        break;
      case "exception":
        dapReason = "exception";
        break;
      case "debugCommand":
        dapReason = "pause";
        break;
      default:
        dapReason = "step";
    }
    let line;
    if (callFrames && callFrames.length > 0) {
      const transpiledLine = callFrames[0].location.lineNumber;
      if (this.sourceMapMappings.transpiledToOriginal.size > 0) {
        const originalLine = this.sourceMapMappings.transpiledToOriginal.get(transpiledLine);
        if (originalLine !== undefined) {
          line = originalLine;
          logger.info(`Stopped: transpiled line ${transpiledLine} -> original line ${line}`);
        } else {
          line = transpiledLine;
          logger.warn(`Transpiled line ${transpiledLine} not in reverse source map, using as-is`);
        }
      } else {
        line = transpiledLine;
      }
    }
    this.sendEvent("stopped", {
      reason: dapReason,
      threadId: 1,
      allThreadsStopped: true,
      line
    });
    this.isStepping = false;
    for (const outputEvent of this.pendingConsoleOutput) {
      this.sendEvent("output", outputEvent);
    }
    this.pendingConsoleOutput = [];
  }
  handleConsoleOutput(params) {
    const type = params.type;
    const args = params.args;
    if (!args)
      return;
    const output = args.map((arg) => {
      if (arg.value !== undefined)
        return String(arg.value);
      if (arg.description)
        return arg.description;
      return "";
    }).join(" ");
    this.sendEvent("output", {
      category: type === "error" ? "stderr" : "stdout",
      output: output + `
`
    });
  }
  formatConsoleParameter(p) {
    if (p.value !== undefined) {
      if (typeof p.value === "string")
        return p.value;
      return String(p.value);
    }
    if (p.subtype === "null")
      return "null";
    if (p.type === "undefined")
      return "undefined";
    if (p.type === "object" && p.preview && p.preview.properties) {
      const props = p.preview.properties;
      const isArray = p.subtype === "array" || p.preview.subtype === "array";
      if (isArray) {
        const items = props.map((prop) => this.formatPreviewValue(prop));
        const suffix = p.preview.overflow ? ", ..." : "";
        return `[${items.join(", ")}${suffix}]`;
      } else {
        const items = props.map((prop) => `${prop.name}: ${this.formatPreviewValue(prop)}`);
        const suffix = p.preview.overflow ? ", ..." : "";
        return `{ ${items.join(", ")}${suffix} }`;
      }
    }
    if (p.type === "function") {
      return p.description || "[Function]";
    }
    if (p.description)
      return p.description;
    return "";
  }
  formatPreviewValue(prop) {
    if (prop.subtype === "null")
      return "null";
    if (prop.type === "undefined")
      return "undefined";
    if (prop.type === "string")
      return `"${prop.value ?? ""}"`;
    if (prop.type === "number" || prop.type === "boolean")
      return prop.value ?? "";
    if (prop.type === "function")
      return "[Function]";
    if (prop.type === "object") {
      if (prop.subtype === "array")
        return prop.value || "[]";
      return prop.value || "{...}";
    }
    return prop.value ?? "";
  }
  handleConsoleMessageAdded(params) {
    const message = params.message;
    if (!message)
      return;
    let output;
    if (message.parameters && message.parameters.length > 0) {
      output = message.parameters.map((p) => this.formatConsoleParameter(p)).join(" ");
    } else {
      output = message.text || "";
    }
    if (output.startsWith("__WINDMILL_RESULT__:")) {
      try {
        const resultJson = output.substring("__WINDMILL_RESULT__:".length);
        this.scriptResult = JSON.parse(resultJson);
        logger.info(`Captured script result: ${resultJson}`);
        this.isStepping = false;
        for (const pendingOutput of this.pendingConsoleOutput) {
          this.sendEvent("output", pendingOutput);
        }
        this.pendingConsoleOutput = [];
        if (!this.terminatedSent) {
          this.terminatedSent = true;
          this.running = false;
          logger.info("Sending terminated event after result capture");
          this.sendEvent("terminated", { result: this.scriptResult });
        }
      } catch (error) {
        logger.error("Failed to parse script result:", error);
      }
      return;
    }
    const category = message.level === "error" || message.level === "warning" ? "stderr" : "stdout";
    const outputEvent = {
      category,
      output: output + `
`
    };
    if (message.line !== undefined && message.url) {
      outputEvent.source = {
        path: message.url,
        name: message.url.split("/").pop() || "script.ts"
      };
      outputEvent.line = message.line;
      if (message.column !== undefined) {
        outputEvent.column = message.column + 1;
      }
    }
    if (this.isStepping) {
      this.pendingConsoleOutput.push(outputEvent);
    } else {
      this.sendEvent("output", outputEvent);
    }
  }
  handleException(params) {
    const exceptionDetails = params.exceptionDetails;
    if (exceptionDetails) {
      const message = exceptionDetails.exception?.description || exceptionDetails.text || "Unknown exception";
      this.sendEvent("output", {
        category: "stderr",
        output: `Exception: ${message}
`
      });
    }
  }
  async handleInitialize(request) {
    const capabilities = {
      supportsConfigurationDoneRequest: true,
      supportsFunctionBreakpoints: false,
      supportsConditionalBreakpoints: false,
      supportsHitConditionalBreakpoints: false,
      supportsEvaluateForHovers: true,
      exceptionBreakpointFilters: [],
      supportsStepBack: false,
      supportsSetVariable: false,
      supportsRestartFrame: false,
      supportsGotoTargetsRequest: false,
      supportsStepInTargetsRequest: false,
      supportsCompletionsRequest: false,
      supportsModulesRequest: false,
      supportsExceptionOptions: false,
      supportsValueFormattingOptions: false,
      supportsExceptionInfoRequest: false,
      supportTerminateDebuggee: true,
      supportsDelayedStackTraceLoading: false,
      supportsLoadedSourcesRequest: false,
      supportsLogPoints: false,
      supportsTerminateThreadsRequest: false,
      supportsSetExpression: false,
      supportsTerminateRequest: true,
      supportsDataBreakpoints: false,
      supportsReadMemoryRequest: false,
      supportsDisassembleRequest: false,
      supportsCancelRequest: false,
      supportsBreakpointLocationsRequest: false
    };
    this.sendResponse(request, true, capabilities);
    this.initialized = true;
    this.sendEvent("initialized");
  }
  async handleSetBreakpoints(request) {
    const args = request.arguments || {};
    const source = args.source;
    const sourcePath = source?.path || "";
    const breakpointsData = args.breakpoints || [];
    const verifiedBreakpoints = [];
    const lineNumbers = [];
    for (const bp of breakpointsData) {
      const line = bp.line;
      lineNumbers.push(line);
      verifiedBreakpoints.push({
        id: verifiedBreakpoints.length + 1,
        verified: true,
        line,
        source: { path: sourcePath }
      });
    }
    this.breakpoints.set(sourcePath, lineNumbers);
    logger.info(`Stored breakpoints at lines ${lineNumbers} for ${sourcePath}`);
    if (this.inspectorWs) {
      await this.applyBreakpointsByUrl();
    }
    this.sendResponse(request, true, { breakpoints: verifiedBreakpoints });
  }
  async handleConfigurationDone(request) {
    this.configured = true;
    this.sendResponse(request);
  }
  async handleLaunch(request) {
    const args = request.arguments || {};
    let code = args.code;
    this.scriptPath = args.program;
    const cwd = args.cwd || process.cwd();
    this.callMain = args.callMain || false;
    this.mainArgs = args.args || {};
    this.envVars = args.env || {};
    if (process.env.BASE_INTERNAL_URL) {
      this.envVars.WM_BASE_URL = process.env.BASE_INTERNAL_URL;
    }
    if (Object.keys(this.envVars).length > 0) {
      logger.info(`Launch with env vars: ${Object.keys(this.envVars).join(", ")}`);
    }
    if (!this.scriptPath && !code) {
      this.sendResponse(request, false, {}, "No program or code specified");
      return;
    }
    if (code) {
      this.nodeModulesPath = await this.prepareDependencies(code) || undefined;
    }
    this.initialPauseDone = false;
    code = `debugger; // Auto-injected by Windmill debugger
${code || ""}`;
    if (this.callMain && code) {
      const argsValues = Object.values(this.mainArgs).map((v) => JSON.stringify(v)).join(", ");
      code = code + `

// Auto-generated call to main entrypoint
` + `globalThis.__windmill_result__ = await main(${argsValues});
` + `console.log("__WINDMILL_RESULT__:" + JSON.stringify(globalThis.__windmill_result__));
` + `// Small delay to ensure console output is delivered via inspector before process exits
` + `await new Promise(r => setTimeout(r, 50));
`;
      logger.info(`Added main() call with args: ${argsValues}`);
    }
    if (code && !this.scriptPath) {
      try {
        this.tempDir = await mkdtemp(join(tmpdir(), "windmill_debug_"));
        this.tempFile = join(this.tempDir, "script.ts");
        await writeFile(this.tempFile, code);
        this.scriptPath = this.tempFile;
        logger.info(`Wrote code to ${this.tempFile}`);
        const lines = code.split(`
`);
        for (let i = 24;i < Math.min(30, lines.length); i++) {
          logger.info(`  Line ${i} (0-idx) / ${i + 1} (1-idx): ${lines[i]?.substring(0, 80)}`);
        }
      } catch (error) {
        this.sendResponse(request, false, {}, `Failed to create temp file: ${error}`);
        return;
      }
    }
    this.sendResponse(request);
    try {
      await this.startBunProcess(cwd);
    } catch (error) {
      this.sendEvent("output", { category: "stderr", output: `Failed to start Bun: ${error}
` });
      this.sendEvent("terminated", { error: String(error) });
    }
  }
  inspectorWsUrl = null;
  inspectorWsUrlPromise = null;
  async prepareDependencies(code, language = "bun") {
    if (!this.windmillPath) {
      logger.info("No windmill binary path configured, skipping dependency preparation");
      return null;
    }
    logger.info(`Preparing dependencies using ${this.windmillPath}`);
    try {
      const input = JSON.stringify({ code, language });
      const proc = spawn({
        cmd: [this.windmillPath, "prepare-deps"],
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe"
      });
      const writer = proc.stdin.getWriter();
      await writer.write(new TextEncoder().encode(input));
      await writer.close();
      const output = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();
      if (stderr) {
        const filteredStderr = stderr.split(`
`).filter((line) => !line.includes("Running in standalone mode")).join(`
`).trim();
        if (filteredStderr) {
          logger.debug(`prepare-deps stderr: ${filteredStderr}`);
        }
      }
      const response = JSON.parse(output.trim().split(`
`).pop() || "{}");
      if (!response.success) {
        logger.error(`prepare-deps failed: ${response.error}`);
        this.sendEvent("output", {
          category: "console",
          output: `Warning: Failed to prepare dependencies: ${response.error}
`
        });
        return null;
      }
      if (response.node_modules_path) {
        logger.info(`Dependencies installed at: ${response.node_modules_path}`);
        this.sendEvent("output", {
          category: "console",
          output: `Dependencies installed at: ${response.node_modules_path}
`
        });
        return response.node_modules_path;
      }
      logger.info("No external dependencies to install");
      return null;
    } catch (error) {
      logger.error(`Failed to prepare dependencies: ${error}`);
      this.sendEvent("output", {
        category: "console",
        output: `Warning: Failed to prepare dependencies: ${error}
`
      });
      return null;
    }
  }
  async startBunProcess(cwd) {
    if (!this.scriptPath) {
      throw new Error("No script path");
    }
    const inspectPort = 9229 + Math.floor(Math.random() * 1000);
    const inspectUrl = `127.0.0.1:${inspectPort}`;
    let cmd = [this.bunPath, `--inspect-wait=${inspectUrl}`, this.scriptPath];
    if (this.nsjailConfig?.enabled) {
      const nsjailCmd = [this.nsjailConfig.binaryPath];
      if (this.nsjailConfig.configPath) {
        nsjailCmd.push("--config", this.nsjailConfig.configPath);
      }
      if (this.nsjailConfig.extraArgs) {
        nsjailCmd.push(...this.nsjailConfig.extraArgs);
      }
      nsjailCmd.push("--cwd", cwd);
      nsjailCmd.push("--");
      nsjailCmd.push(...cmd);
      cmd = nsjailCmd;
      logger.info(`Starting Bun with nsjail: ${cmd.join(" ")}`);
    } else {
      logger.info(`Starting Bun with --inspect-wait=${inspectUrl}`);
    }
    const wsUrlPromise = new Promise((resolve, reject) => {
      this.inspectorWsUrlPromise = { resolve, reject };
      setTimeout(() => {
        if (this.inspectorWsUrlPromise) {
          reject(new Error("Timeout waiting for inspector WebSocket URL"));
          this.inspectorWsUrlPromise = null;
        }
      }, 1e4);
    });
    const envVars = {
      PATH: process.env.PATH || "/usr/bin:/bin",
      HOME: process.env.HOME,
      ...this.envVars
    };
    if (this.nodeModulesPath) {
      envVars.NODE_PATH = this.nodeModulesPath;
      logger.info(`Setting NODE_PATH=${this.nodeModulesPath}`);
    }
    this.process = spawn({
      cmd,
      cwd,
      stdout: "pipe",
      stderr: "pipe",
      env: envVars
    });
    this.readStderrForInspectorUrl(this.process.stderr);
    const wsUrl = await wsUrlPromise;
    logger.info(`Got inspector WebSocket URL: ${wsUrl}`);
    await this.connectToInspector(wsUrl);
    this.process.exited.then(async (exitCode) => {
      logger.info(`Bun process exited with code: ${exitCode}`);
      this.running = false;
      if (!this.terminatedSent) {
        this.terminatedSent = true;
        logger.info(`Sending terminated event with result: ${JSON.stringify(this.scriptResult)}`);
        try {
          this.sendEvent("terminated", this.scriptResult !== undefined ? { result: this.scriptResult } : {});
          logger.info("Terminated event sent successfully");
        } catch (error) {
          logger.error("Failed to send terminated event:", error);
        }
      }
      await this.cleanup();
    }).catch((error) => {
      logger.error("Error in process.exited handler:", error);
    });
  }
  async readStream(stream, category) {
    if (!stream)
      return;
    const reader = stream.getReader();
    const decoder = new TextDecoder;
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done)
          break;
        const text = decoder.decode(value);
        this.sendEvent("output", { category, output: text });
      }
    } catch (error) {
      logger.debug(`Stream ${category} ended:`, error);
    }
  }
  async readStderrForInspectorUrl(stream) {
    if (!stream)
      return;
    const reader = stream.getReader();
    const decoder = new TextDecoder;
    let buffer = "";
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done)
          break;
        const text = decoder.decode(value);
        buffer += text;
        const wsMatch = buffer.match(/ws:\/\/[\d.]+:\d+\/[a-z0-9]+/i);
        if (wsMatch && this.inspectorWsUrlPromise) {
          const wsUrl = wsMatch[0];
          logger.info(`Found inspector WebSocket URL in stderr: ${wsUrl}`);
          this.inspectorWsUrl = wsUrl;
          this.inspectorWsUrlPromise.resolve(wsUrl);
          this.inspectorWsUrlPromise = null;
        }
        if (!text.includes("Bun Inspector") && !text.includes("Listening:") && !text.includes("ws://") && !text.includes("debug.bun.sh")) {
          this.sendEvent("output", { category: "stderr", output: text });
        }
      }
    } catch (error) {
      logger.debug("Stderr stream ended:", error);
    }
  }
  async connectToInspector(wsUrl) {
    logger.info(`Connecting to inspector at ${wsUrl}`);
    return new Promise((resolve, reject) => {
      this.inspectorWs = new WebSocket(wsUrl);
      const timeout = setTimeout(() => {
        reject(new Error("Inspector connection timeout"));
      }, 5000);
      this.inspectorWs.onopen = async () => {
        clearTimeout(timeout);
        logger.info("Connected to inspector");
        try {
          await this.sendInspectorCommand("Inspector.enable", {});
          await this.sendInspectorCommand("Console.enable", {});
          await this.sendInspectorCommand("Debugger.enable", {});
          await this.sendInspectorCommand("Runtime.enable", {});
          await this.sendInspectorCommand("Debugger.setBreakpointsActive", { active: true });
          await this.sendInspectorCommand("Debugger.setPauseOnDebuggerStatements", { enabled: true });
          await this.sendInspectorCommand("Debugger.setPauseOnExceptions", {
            state: "uncaught"
          });
          await this.applyBreakpointsByUrl();
          this.running = true;
          logger.info("Starting script execution with Inspector.initialized...");
          await this.sendInspectorCommand("Inspector.initialized", {});
          resolve();
        } catch (error) {
          reject(error);
        }
      };
      this.inspectorWs.onmessage = (event) => {
        this.handleInspectorMessage(event.data);
      };
      this.inspectorWs.onerror = (error) => {
        logger.error("Inspector WebSocket error:", error);
      };
      this.inspectorWs.onclose = () => {
        logger.info("Inspector WebSocket closed");
        this.inspectorWs = null;
        if (!this.terminatedSent) {
          this.terminatedSent = true;
          this.running = false;
          logger.info(`Inspector closed - sending terminated event with result: ${JSON.stringify(this.scriptResult)}`);
          try {
            this.sendEvent("terminated", this.scriptResult !== undefined ? { result: this.scriptResult } : {});
            logger.info("Terminated event sent successfully via inspector close");
          } catch (error) {
            logger.error("Failed to send terminated event on inspector close:", error);
          }
        }
      };
    });
  }
  async handleThreads(request) {
    this.sendResponse(request, true, {
      threads: [{ id: 1, name: "MainThread" }]
    });
  }
  async handleStackTrace(request) {
    logger.info(`handleStackTrace: callFrames.length=${this.callFrames.length}`);
    const stackFrames = [];
    for (let i = 0;i < this.callFrames.length; i++) {
      const frame = this.callFrames[i];
      const script = this.scripts.get(frame.location.scriptId);
      logger.info(`  frame ${i}: functionName="${frame.functionName}", scriptId=${frame.location.scriptId}, lineNumber=${frame.location.lineNumber}, scriptUrl=${script?.url}`);
      if (!script?.url || !this.scriptPath) {
        logger.info(`  skipping frame ${i}: no script URL or scriptPath`);
        continue;
      }
      const isUserScript = script.url.endsWith(this.scriptPath.split("/").pop());
      if (!isUserScript) {
        logger.info(`  skipping frame ${i}: not from user script (${script.url})`);
        continue;
      }
      const transpiledLine = frame.location.lineNumber;
      let frameLine = transpiledLine;
      if (this.sourceMapMappings.transpiledToOriginal.size > 0) {
        const originalLine = this.sourceMapMappings.transpiledToOriginal.get(transpiledLine);
        if (originalLine !== undefined) {
          frameLine = originalLine;
        }
      }
      stackFrames.push({
        id: i + 1,
        name: frame.functionName || "<module>",
        source: {
          path: script.url || this.scriptPath || "<unknown>",
          name: script.url?.split("/").pop() || "script.ts"
        },
        line: frameLine,
        column: frame.location.columnNumber + 1
      });
    }
    logger.info(`handleStackTrace: returning ${stackFrames.length} frames`);
    this.sendResponse(request, true, {
      stackFrames,
      totalFrames: stackFrames.length
    });
  }
  async handleScopes(request) {
    const args = request.arguments || {};
    const frameId = args.frameId || 1;
    const frameIndex = frameId - 1;
    logger.info(`handleScopes: frameId=${frameId}, frameIndex=${frameIndex}, callFrames.length=${this.callFrames.length}`);
    const frame = this.callFrames[frameIndex];
    if (!frame) {
      logger.warn(`handleScopes: No frame at index ${frameIndex}`);
      this.sendResponse(request, true, { scopes: [] });
      return;
    }
    logger.info(`handleScopes: frame.scopeChain has ${frame.scopeChain.length} scopes`);
    for (const s of frame.scopeChain) {
      logger.info(`  scope: type=${s.type}, name=${s.name}, hasObjectId=${!!s.object.objectId}`);
    }
    const scopes = [];
    for (const scope of frame.scopeChain) {
      const ref = this.nextVarRef();
      const objectId = scope.object.objectId;
      if (objectId) {
        this.scopesMap.set(ref, { type: scope.type, objectId, frameIndex });
        let name;
        if (scope.name) {
          name = scope.name;
        } else {
          name = scope.type.charAt(0).toUpperCase() + scope.type.slice(1);
        }
        scopes.push({
          name,
          variablesReference: ref,
          expensive: scope.type === "global"
        });
        logger.info(`  Added scope: name=${name}, ref=${ref}, type=${scope.type}`);
      } else {
        logger.warn(`  Scope ${scope.type} has no objectId, skipping`);
      }
    }
    logger.info(`handleScopes: returning ${scopes.length} scopes`);
    this.sendResponse(request, true, { scopes });
  }
  async handleVariables(request) {
    const args = request.arguments || {};
    const variablesRef = args.variablesReference || 0;
    logger.info(`handleVariables: variablesRef=${variablesRef}`);
    const scopeInfo = this.scopesMap.get(variablesRef);
    const objectId = this.objectsMap.get(variablesRef) || scopeInfo?.objectId;
    logger.info(`handleVariables: scopeInfo=${JSON.stringify(scopeInfo)}, objectId=${objectId}`);
    if (!objectId) {
      logger.warn(`handleVariables: No objectId found for ref ${variablesRef}`);
      this.sendResponse(request, true, { variables: [] });
      return;
    }
    try {
      const scopeType = scopeInfo?.type || "unknown";
      const isGlobalScope = scopeType === "global";
      logger.info(`handleVariables: calling Runtime.getProperties for objectId=${objectId}, scopeType=${scopeType}`);
      const response = await this.sendInspectorCommand("Runtime.getProperties", {
        objectId,
        ownProperties: true,
        generatePreview: true
      });
      const allProps = response.result?.properties || [];
      logger.info(`handleVariables: all property names: ${allProps.map((p) => p.name).join(", ")}`);
      logger.info(`handleVariables: got response with ${response.result?.properties?.length || 0} properties for scope type: ${scopeType}`);
      const variables = [];
      const properties = response.result?.properties || [];
      for (const prop of properties) {
        if (!prop.value)
          continue;
        if (prop.name.startsWith("__") && prop.name !== "__windmill_result__")
          continue;
        if (prop.value.type === "function" && prop.value.description?.includes("[native code]"))
          continue;
        const builtInNames = new Set([
          "NaN",
          "Infinity",
          "undefined",
          "globalThis",
          "global",
          "self",
          "window",
          "console",
          "Bun",
          "process",
          "navigator",
          "performance",
          "crypto",
          "Loader",
          "onmessage",
          "onerror",
          "toString",
          "toLocaleString",
          "valueOf",
          "hasOwnProperty",
          "propertyIsEnumerable",
          "isPrototypeOf",
          "constructor",
          "Reflect",
          "JSON",
          "Math",
          "Atomics",
          "Intl",
          "WebAssembly",
          "Proxy",
          "Object",
          "Array",
          "Function",
          "Boolean",
          "Symbol",
          "Number",
          "BigInt",
          "String",
          "RegExp",
          "Date",
          "Promise",
          "Map",
          "Set",
          "WeakMap",
          "WeakSet",
          "Error",
          "TypeError",
          "RangeError",
          "SyntaxError",
          "ReferenceError",
          "EvalError",
          "URIError",
          "AggregateError",
          "ArrayBuffer",
          "DataView",
          "Int8Array",
          "Uint8Array",
          "Uint8ClampedArray",
          "Int16Array",
          "Uint16Array",
          "Int32Array",
          "Uint32Array",
          "Float32Array",
          "Float64Array",
          "BigInt64Array",
          "BigUint64Array",
          "SharedArrayBuffer"
        ]);
        if (builtInNames.has(prop.name))
          continue;
        if (prop.value.type === "function" && /^[A-Z][a-zA-Z0-9]*$/.test(prop.name))
          continue;
        let value;
        let varRef = 0;
        let displayType = prop.value.type;
        const className = prop.value.className;
        logger.debug(`Variable ${prop.name}: type=${prop.value.type}, className=${className}, description=${prop.value.description}, value=${JSON.stringify(prop.value.value)}`);
        if (prop.value.type === "object" && prop.value.objectId) {
          if (className === "String" && prop.value.description) {
            value = prop.value.description.startsWith('"') ? prop.value.description : `"${prop.value.description}"`;
            displayType = "string";
          } else if (className === "Number" && prop.value.description) {
            value = prop.value.description;
            displayType = "number";
          } else if (className === "Boolean" && prop.value.description) {
            value = prop.value.description;
            displayType = "boolean";
          } else {
            varRef = this.nextVarRef();
            this.objectsMap.set(varRef, prop.value.objectId);
            if (prop.value.preview && prop.value.preview.properties) {
              const previewProps = prop.value.preview.properties;
              const isArray = prop.value.subtype === "array" || prop.value.preview.subtype === "array";
              if (isArray) {
                const items = previewProps.map((p) => this.formatPreviewValue(p));
                const suffix = prop.value.preview.overflow ? ", ..." : "";
                value = `[${items.join(", ")}${suffix}]`;
              } else {
                const items = previewProps.map((p) => `${p.name}: ${this.formatPreviewValue(p)}`);
                const suffix = prop.value.preview.overflow ? ", ..." : "";
                value = `{ ${items.join(", ")}${suffix} }`;
              }
            } else {
              value = prop.value.description || `[${prop.value.subtype || className || "Object"}]`;
            }
          }
        } else if (prop.value.type === "string") {
          value = prop.value.value !== undefined ? JSON.stringify(prop.value.value) : prop.value.description || '""';
          displayType = "string";
        } else if (prop.value.value !== undefined) {
          value = JSON.stringify(prop.value.value);
        } else {
          value = prop.value.description || String(prop.value.type);
        }
        variables.push({
          name: prop.name,
          value,
          type: displayType,
          variablesReference: varRef
        });
      }
      logger.info(`handleVariables: returning ${variables.length} variables (filtered from ${properties.length})`);
      this.sendResponse(request, true, { variables });
    } catch (error) {
      logger.error("Failed to get variables:", error);
      this.sendResponse(request, true, { variables: [] });
    }
  }
  async handleEvaluate(request) {
    const args = request.arguments || {};
    const expression = args.expression || "";
    const frameId = args.frameId;
    try {
      let result;
      let varRef = 0;
      if (frameId !== undefined && this.callFrames[frameId - 1]) {
        const frame = this.callFrames[frameId - 1];
        const response = await this.sendInspectorCommand("Debugger.evaluateOnCallFrame", {
          callFrameId: frame.callFrameId,
          expression,
          returnByValue: true
        });
        const evalResult = response.result?.result;
        if (evalResult) {
          if (evalResult.value !== undefined) {
            result = JSON.stringify(evalResult.value);
          } else if (evalResult.objectId) {
            varRef = this.nextVarRef();
            this.objectsMap.set(varRef, evalResult.objectId);
            result = evalResult.description || "[Object]";
          } else {
            result = evalResult.description || String(evalResult.type);
          }
        } else {
          result = "undefined";
        }
      } else {
        const response = await this.sendInspectorCommand("Runtime.evaluate", {
          expression,
          returnByValue: true
        });
        const evalResult = response.result?.result;
        result = evalResult?.value !== undefined ? JSON.stringify(evalResult.value) : evalResult?.description || "undefined";
      }
      this.sendResponse(request, true, { result, variablesReference: varRef });
    } catch (error) {
      this.sendResponse(request, true, {
        result: `Error: ${error}`,
        variablesReference: 0
      });
    }
  }
  async handleContinue(request) {
    try {
      this.isStepping = true;
      this.pausedAtBreakpoint = false;
      await this.sendInspectorCommand("Debugger.resume", {});
      this.sendResponse(request, true, { allThreadsContinued: true });
    } catch (error) {
      this.isStepping = false;
      this.sendResponse(request, false, {}, String(error));
    }
  }
  async handleNext(request) {
    try {
      this.isStepping = true;
      this.pausedAtBreakpoint = false;
      await this.sendInspectorCommand("Debugger.stepOver", {});
      this.sendResponse(request);
    } catch (error) {
      this.isStepping = false;
      this.sendResponse(request, false, {}, String(error));
    }
  }
  async handleStepIn(request) {
    try {
      this.isStepping = true;
      this.pausedAtBreakpoint = false;
      await this.sendInspectorCommand("Debugger.stepInto", {});
      this.sendResponse(request);
    } catch (error) {
      this.isStepping = false;
      this.sendResponse(request, false, {}, String(error));
    }
  }
  async handleStepOut(request) {
    try {
      this.isStepping = true;
      this.pausedAtBreakpoint = false;
      await this.sendInspectorCommand("Debugger.stepOut", {});
      this.sendResponse(request);
    } catch (error) {
      this.isStepping = false;
      this.sendResponse(request, false, {}, String(error));
    }
  }
  async handlePause(request) {
    try {
      await this.sendInspectorCommand("Debugger.pause", {});
      this.sendResponse(request);
    } catch (error) {
      this.sendResponse(request, false, {}, String(error));
    }
  }
  async handleDisconnect(request) {
    this.running = false;
    await this.cleanup();
    this.sendResponse(request);
  }
  async handleTerminate(request) {
    this.running = false;
    const shouldSendTerminated = !this.terminatedSent;
    this.terminatedSent = true;
    await this.cleanup();
    this.sendResponse(request);
    if (shouldSendTerminated) {
      this.sendEvent("terminated", this.scriptResult !== undefined ? { result: this.scriptResult } : {});
    }
  }
  async cleanup() {
    if (this.inspectorWs) {
      this.inspectorWs.close();
      this.inspectorWs = null;
    }
    if (this.process) {
      this.process.kill();
      this.process = null;
    }
    if (this.tempFile) {
      try {
        await unlink(this.tempFile);
      } catch {}
      this.tempFile = null;
    }
    if (this.tempDir) {
      try {
        await rmdir(this.tempDir);
      } catch {}
      this.tempDir = null;
    }
  }
  async handleRequest(request) {
    const command = request.command || "";
    logger.debug(`Handling command: ${command}`);
    const handlers = {
      initialize: (req) => this.handleInitialize(req),
      setBreakpoints: (req) => this.handleSetBreakpoints(req),
      configurationDone: (req) => this.handleConfigurationDone(req),
      launch: (req) => this.handleLaunch(req),
      threads: (req) => this.handleThreads(req),
      stackTrace: (req) => this.handleStackTrace(req),
      scopes: (req) => this.handleScopes(req),
      variables: (req) => this.handleVariables(req),
      evaluate: (req) => this.handleEvaluate(req),
      continue: (req) => this.handleContinue(req),
      next: (req) => this.handleNext(req),
      stepIn: (req) => this.handleStepIn(req),
      stepOut: (req) => this.handleStepOut(req),
      pause: (req) => this.handlePause(req),
      disconnect: (req) => this.handleDisconnect(req),
      terminate: (req) => this.handleTerminate(req)
    };
    const handler = handlers[command];
    if (handler) {
      await handler(request);
    } else {
      logger.warn(`Unhandled command: ${command}`);
      this.sendResponse(request, false, {}, `Unsupported command: ${command}`);
    }
  }
}
var sessions = new Map;
var args = process.argv.slice(2);
var host = "localhost";
var port = 5680;
var windmillPath;
for (let i = 0;i < args.length; i++) {
  if (args[i] === "--host" && args[i + 1]) {
    host = args[i + 1];
    i++;
  } else if (args[i] === "--port" && args[i + 1]) {
    port = parseInt(args[i + 1], 10);
    i++;
  } else if (args[i] === "--windmill" && args[i + 1]) {
    windmillPath = args[i + 1];
    i++;
  }
}
if (windmillPath) {
  logger.info(`Windmill binary path: ${windmillPath}`);
}
logger.info(`Starting DAP WebSocket server on ws://${host}:${port}`);
var server = Bun.serve({
  hostname: host,
  port,
  fetch(req, server2) {
    if (server2.upgrade(req)) {
      return;
    }
    return new Response("DAP WebSocket Server for Bun/TypeScript", { status: 200 });
  },
  websocket: {
    open(ws) {
      logger.info("New client connected");
      const wsWrapper = {
        send: (data) => ws.send(data),
        close: () => ws.close(),
        readyState: WebSocket.OPEN,
        CONNECTING: WebSocket.CONNECTING,
        OPEN: WebSocket.OPEN,
        CLOSING: WebSocket.CLOSING,
        CLOSED: WebSocket.CLOSED
      };
      const session = new DebugSession(wsWrapper, { windmillPath });
      sessions.set(ws, session);
    },
    async message(ws, message) {
      const session = sessions.get(ws);
      if (!session)
        return;
      try {
        const data = JSON.parse(message);
        logger.debug("Received:", JSON.stringify(data));
        if (data.type === "request") {
          await session.handleRequest(data);
        }
      } catch (error) {
        logger.error("Error handling message:", error);
      }
    },
    close(ws) {
      logger.info("Client disconnected");
      sessions.delete(ws);
    }
  }
});
logger.info(`Server started on ${server.url}`);
export {
  DebugSession
};
