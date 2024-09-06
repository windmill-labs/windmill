/**
 * Logging library with the support for terminal and file outputs. Also provides
 * interfaces for building custom loggers.
 *
 * ## Loggers
 *
 * Loggers are objects that you interact with. When you use a logger method it
 * constructs a `LogRecord` and passes it down to its handlers for output. To
 * create custom loggers, specify them in `loggers` when calling `log.setup`.
 *
 * ## Custom message format
 *
 * If you want to override default format of message you can define `formatter`
 * option for handler. It can a function that takes `LogRecord`
 * as argument and outputs string.
 *
 * The default log format is `{levelName} {msg}`.
 *
 * ### Logging Structured JSON Lines
 *
 * To output logs in a structured JSON format you can configure most handlers
 * with a formatter that produces a JSON string. Either use the premade
 * `log.formatters.jsonFormatter` or write your own function that takes a
 * {@linkcode LogRecord} and returns a JSON.stringify'd object.
 * If you want the log to go to stdout then use {@linkcode ConsoleHandler} with
 * the configuration `useColors: false` to turn off the ANSI terminal colours.
 *
 * ```ts
 * import * as log from "@std/log";
 *
 * log.setup({
 *   handlers: {
 *     default: new log.ConsoleHandler("DEBUG", {
 *       formatter: log.formatters.jsonFormatter,
 *       useColors: false,
 *     }),
 *   },
 * });
 * ```
 *
 * The first argument passed to a log function is always treated as the
 * message and will be stringified differently. To have arguments JSON.stringify'd
 * you must pass them after the first.
 *
 * ```ts
 * import * as log from "@std/log";
 *
 * log.info("This is the message", { thisWillBe: "JSON.stringify'd"});
 * // {"level":"INFO","datetime":1702501580505,"message":"This is the message","args":{"thisWillBe":"JSON.stringify'd"}}
 *
 * log.info({ thisWontBe: "JSON.stringify'd"}, "This is an argument");
 * // {"level":"INFO","datetime":1702501580505,"message":"{\"thisWontBe\":\"JSON.stringify'd\"}","args":"This is an argument"}
 * ```
 *
 * ## Inline Logging
 *
 * Log functions return the data passed in the `msg` parameter. Data is returned
 * regardless if the logger actually logs it.
 *
 * ## Lazy Log Evaluation
 *
 * Some log statements are expensive to compute. In these cases, you can use
 * lazy log evaluation to prevent the computation taking place if the logger
 * won't log the message.
 *
 * > NOTE: When using lazy log evaluation, `undefined` will be returned if the
 * > resolver function is not called because the logger won't log it. It is an
 * > antipattern use lazy evaluation with inline logging because the return value
 * > depends on the current log level.
 *
 * ## For module authors
 *
 * The authors of public modules can let the users display the internal logs of the
 * module by using a custom logger:
 *
 * ```ts
 * import { getLogger } from "@std/log";
 *
 * function logger() {
 *   return getLogger("my-awesome-module");
 * }
 *
 * export function sum(a: number, b: number) {
 *   logger().debug(`running ${a} + ${b}`);
 *   return a + b;
 * }
 *
 * export function mult(a: number, b: number) {
 *   logger().debug(`running ${a} * ${b}`);
 *   return a * b;
 * }
 * ```
 *
 * The user of the module can then display the internal logs with:
 *
 * ```ts, ignore
 * import * as log from "@std/log";
 * import { sum } from "<the-awesome-module>/mod.ts";
 *
 * log.setup({
 *   handlers: {
 *     console: new log.ConsoleHandler("DEBUG"),
 *   },
 *
 *   loggers: {
 *     "my-awesome-module": {
 *       level: "DEBUG",
 *       handlers: ["console"],
 *     },
 *   },
 * });
 *
 * sum(1, 2); // prints "running 1 + 2" to the console
 * ```
 *
 * Please note that, due to the order of initialization of the loggers, the
 * following won't work:
 *
 * ```ts
 * import { getLogger } from "@std/log";
 *
 * const logger = getLogger("my-awesome-module");
 *
 * export function sum(a: number, b: number) {
 *   logger.debug(`running ${a} + ${b}`); // no message will be logged, because getLogger() was called before log.setup()
 *   return a + b;
 * }
 * ```
 *
 * @example
 * ```ts
 * import * as log from "@std/log";
 *
 * // Simple default logger out of the box. You can customize it
 * // by overriding logger and handler named "default", or providing
 * // additional logger configurations. You can log any data type.
 * log.debug("Hello world");
 * log.info(123456);
 * log.warn(true);
 * log.error({ foo: "bar", fizz: "bazz" });
 * log.critical("500 Internal server error");
 *
 * // custom configuration with 2 loggers (the default and `tasks` loggers).
 * log.setup({
 *   handlers: {
 *     console: new log.ConsoleHandler("DEBUG"),
 *
 *     file: new log.FileHandler("WARN", {
 *       filename: "./log.txt",
 *       // you can change format of output message using any keys in `LogRecord`.
 *       formatter: (record) => `${record.levelName} ${record.msg}`,
 *     }),
 *   },
 *
 *   loggers: {
 *     // configure default logger available via short-hand methods above.
 *     default: {
 *       level: "DEBUG",
 *       handlers: ["console", "file"],
 *     },
 *
 *     tasks: {
 *       level: "ERROR",
 *       handlers: ["console"],
 *     },
 *   },
 * });
 *
 * let logger;
 *
 * // get default logger.
 * logger = log.getLogger();
 * logger.debug("fizz"); // logs to `console`, because `file` handler requires "WARN" level.
 * logger.warn(41256); // logs to both `console` and `file` handlers.
 *
 * // get custom logger
 * logger = log.getLogger("tasks");
 * logger.debug("fizz"); // won't get output because this logger has "ERROR" level.
 * logger.error({ productType: "book", value: "126.11" }); // log to `console`.
 *
 * // if you try to use a logger that hasn't been configured
 * // you're good to go, it gets created automatically with level set to 0
 * // so no message is logged.
 * const unknownLogger = log.getLogger("mystery");
 * unknownLogger.info("foobar"); // no-op
 * ```
 *
 * @example
 * Custom message format example
 * ```ts
 * import * as log from "@std/log";
 *
 * log.setup({
 *   handlers: {
 *     stringFmt: new log.ConsoleHandler("DEBUG", {
 *       formatter: (record) => `[${record.levelName}] ${record.msg}`,
 *     }),
 *
 *     functionFmt: new log.ConsoleHandler("DEBUG", {
 *       formatter: (logRecord) => {
 *         let msg = `${logRecord.level} ${logRecord.msg}`;
 *
 *         logRecord.args.forEach((arg, index) => {
 *           msg += `, arg${index}: ${arg}`;
 *         });
 *
 *         return msg;
 *       },
 *     }),
 *
 *     anotherFmt: new log.ConsoleHandler("DEBUG", {
 *       formatter: (record) => `[${record.loggerName}] - ${record.levelName} ${record.msg}`,
 *     }),
 *   },
 *
 *   loggers: {
 *     default: {
 *       level: "DEBUG",
 *       handlers: ["stringFmt", "functionFmt"],
 *     },
 *     dataLogger: {
 *       level: "INFO",
 *       handlers: ["anotherFmt"],
 *     },
 *   },
 * });
 *
 * // calling:
 * log.debug("Hello, world!", 1, "two", [3, 4, 5]);
 * // results in: [DEBUG] Hello, world!
 * // output from "stringFmt" handler.
 * // 10 Hello, world!, arg0: 1, arg1: two, arg3: [3, 4, 5] // output from "functionFmt" formatter.
 *
 * // calling:
 * log.getLogger("dataLogger").error("oh no!");
 * // results in:
 * // [dataLogger] - ERROR oh no! // output from anotherFmt handler.
 * ```

 *
 * @example
 * JSON to stdout with no color example
 * ```ts
 * import * as log from "@std/log";
 *
 * log.setup({
 *   handlers: {
 *     jsonStdout: new log.ConsoleHandler("DEBUG", {
 *       formatter: log.formatters.jsonFormatter,
 *       useColors: false,
 *     }),
 *   },
 *
 *   loggers: {
 *     default: {
 *       level: "DEBUG",
 *       handlers: ["jsonStdout"],
 *     },
 *   },
 * });
 *
 * // calling:
 * log.info("Hey");
 * // results in:
 * // {"level":"INFO","datetime":1702481922294,"message":"Hey"}
 *
 * // calling:
 * log.info("Hey", { product: "nail" });
 * // results in:
 * // {"level":"INFO","datetime":1702484111115,"message":"Hey","args":{"product":"nail"}}
 *
 * // calling:
 * log.info("Hey", 1, "two", [3, 4, 5]);
 * // results in:
 * // {"level":"INFO","datetime":1702481922294,"message":"Hey","args":[1,"two",[3,4,5]]}
 * ```
 *
 * @example
 * Custom JSON example
 * ```ts
 * import * as log from "@std/log";
 *
 * log.setup({
 *   handlers: {
 *     customJsonFmt: new log.ConsoleHandler("DEBUG", {
 *       formatter: (record) => JSON.stringify({
 *         lvl: record.level,
 *         msg: record.msg,
 *         time: record.datetime.toISOString(),
 *         name: record.loggerName,
 *       }),
 *       useColors: false,
 *     }),
 *   },
 *
 *   loggers: {
 *     default: {
 *       level: "DEBUG",
 *       handlers: ["customJsonFmt"],
 *     },
 *   },
 * });
 *
 * // calling:
 * log.info("complete");
 * // results in:
 * // {"lvl":20,"msg":"complete","time":"2023-12-13T16:38:27.328Z","name":"default"}
 * ```
 *
 * @example
 * Inline Logging
 * ```ts
 * import * as logger from "@std/log";
 *
 * const stringData: string = logger.debug("hello world");
 * const booleanData: boolean = logger.debug(true, 1, "abc");
 * const fn = (): number => {
 *   return 123;
 * };
 * const resolvedFunctionData: number = logger.debug(fn());
 * console.log(stringData); // 'hello world'
 * console.log(booleanData); // true
 * console.log(resolvedFunctionData); // 123
 * ```
 *
 * @example
 * Lazy Log Evaluation
 * ```ts
 * import * as log from "@std/log";
 *
 * log.setup({
 *   handlers: {
 *     console: new log.ConsoleHandler("DEBUG"),
 *   },
 *
 *   loggers: {
 *     tasks: {
 *       level: "ERROR",
 *       handlers: ["console"],
 *     },
 *   },
 * });
 *
 * function someExpensiveFn(num: number, bool: boolean) {
 *   // do some expensive computation
 * }
 *
 * // not logged, as debug < error.
 * const data = log.debug(() => someExpensiveFn(5, true));
 * console.log(data); // undefined
 * ```
 *
 * Handlers are responsible for actual output of log messages. When a handler is
 * called by a logger, it firstly checks that {@linkcode LogRecord}'s level is
 * not lower than level of the handler. If level check passes, handlers formats
 * log record into string and outputs it to target.
 *
 * ## Custom handlers
 *
 * Custom handlers can be implemented by subclassing {@linkcode BaseHandler} or
 * {@linkcode WriterHandler}.
 *
 * {@linkcode BaseHandler} is bare-bones handler that has no output logic at all,
 *
 * {@linkcode WriterHandler} is an abstract class that supports any target with
 * `Writer` interface.
 *
 * During setup async hooks `setup` and `destroy` are called, you can use them
 * to open and close file/HTTP connection or any other action you might need.
 *
 * For examples check source code of {@linkcode FileHandler}`
 * and {@linkcode TestHandler}.
 *
 * @module
 */
export * from "./base_handler.js";
export * from "./console_handler.js";
export * from "./file_handler.js";
export * from "./rotating_file_handler.js";
export * from "./levels.js";
export * from "./logger.js";
export * from "./formatters.js";
export * from "./critical.js";
export * from "./debug.js";
export * from "./error.js";
export * from "./get_logger.js";
export * from "./info.js";
export * from "./setup.js";
export * from "./warn.js";
//# sourceMappingURL=mod.d.ts.map