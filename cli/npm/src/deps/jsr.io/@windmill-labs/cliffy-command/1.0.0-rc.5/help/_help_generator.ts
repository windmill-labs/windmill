import { Table } from "../../../cliffy-table/1.0.0-rc.5/mod.js";
import {
  bold,
  brightBlue,
  brightMagenta,
  dim,
  getColorEnabled,
  green,
  italic,
  red,
  setColorEnabled,
  yellow,
} from "../../../../@std/fmt/0.225.6/colors.js";
import { inspect } from "../../../cliffy-internal/1.0.0-rc.5/runtime/inspect.js";
import {
  dedent,
  getDescription,
  getFlag,
  parseArgumentsDefinition,
} from "../_utils.js";
import type { Command } from "../command.js";
import { Type } from "../type.js";
import type { Argument, EnvVar, Example, Option } from "../types.js";

export interface HelpOptions {
  types?: boolean;
  hints?: boolean;
  colors?: boolean;
  long?: boolean;
}

interface OptionGroup {
  name?: string;
  options: Array<Option>;
}

/** Help text generator. */
export class HelpGenerator {
  private indent = 2;
  private options: Required<HelpOptions>;

  /** Generate help text for given command. */
  public static generate(cmd: Command, options?: HelpOptions): string {
    return new HelpGenerator(cmd, options).generate();
  }

  private constructor(
    private cmd: Command,
    options: HelpOptions = {},
  ) {
    this.options = {
      types: false,
      hints: true,
      colors: true,
      long: false,
      ...options,
    };
  }

  private generate(): string {
    const areColorsEnabled = getColorEnabled();
    setColorEnabled(this.options.colors);

    const result = this.generateHeader() +
      this.generateMeta() +
      this.generateDescription() +
      this.generateOptions() +
      this.generateCommands() +
      this.generateEnvironmentVariables() +
      this.generateExamples();

    setColorEnabled(areColorsEnabled);

    return result;
  }

  private generateHeader(): string {
    const usage = this.cmd.getUsage();
    const rows = [
      [
        bold("Usage:"),
        brightMagenta(
          this.cmd.getPath() +
            (usage ? " " + highlightArguments(usage, this.options.types) : ""),
        ),
      ],
    ];
    const version: string | undefined = this.cmd.getVersion();
    if (version) {
      rows.push([bold("Version:"), yellow(`${this.cmd.getVersion()}`)]);
    }
    return "\n" +
      Table.from(rows)
        .padding(1)
        .toString() +
      "\n";
  }

  private generateMeta(): string {
    const meta = Object.entries(this.cmd.getMeta());
    if (!meta.length) {
      return "";
    }

    const rows = [];
    for (const [name, value] of meta) {
      rows.push([bold(`${name}: `) + value]);
    }

    return "\n" +
      Table.from(rows)
        .padding(1)
        .toString() +
      "\n";
  }

  private generateDescription(): string {
    if (!this.cmd.getDescription()) {
      return "";
    }
    return this.label("Description") +
      Table.from([
        [dedent(this.cmd.getDescription())],
      ])
        .indent(this.indent)
        .maxColWidth(140)
        .padding(1)
        .toString() +
      "\n";
  }

  private generateOptions(): string {
    const options = this.cmd.getOptions(false);
    if (!options.length) {
      return "";
    }

    let groups: Array<OptionGroup> = [];
    const hasGroups = options.some((option) => option.groupName);
    if (hasGroups) {
      for (const option of options) {
        let group = groups.find((group) => group.name === option.groupName);
        if (!group) {
          group = {
            name: option.groupName,
            options: [],
          };
          groups.push(group);
        }
        group.options.push(option);
      }
    } else {
      groups = [{
        name: "Options",
        options,
      }];
    }

    let result = "";
    for (const group of groups) {
      result += this.generateOptionGroup(group);
    }

    return result;
  }

  private generateOptionGroup(group: OptionGroup): string {
    if (!group.options.length) {
      return "";
    }
    const hasTypeDefinitions = !!group.options.find((option) =>
      !!option.typeDefinition
    );

    if (hasTypeDefinitions) {
      return this.label(group.name ?? "Options") +
        Table.from([
          ...group.options.map((option: Option) => [
            option.flags.map((flag) => brightBlue(flag)).join(", "),
            highlightArguments(
              option.typeDefinition || "",
              this.options.types,
            ),
            red(bold("-")),
            getDescription(option.description, !this.options.long),
            this.generateHints(option),
          ]),
        ])
          .padding([2, 2, 1, 2])
          .indent(this.indent)
          .maxColWidth([60, 60, 1, 80, 60])
          .toString() +
        "\n";
    }

    return this.label(group.name ?? "Options") +
      Table.from([
        ...group.options.map((option: Option) => [
          option.flags.map((flag) => brightBlue(flag)).join(", "),
          red(bold("-")),
          getDescription(option.description, !this.options.long),
          this.generateHints(option),
        ]),
      ])
        .indent(this.indent)
        .maxColWidth([60, 1, 80, 60])
        .padding([2, 1, 2])
        .toString() +
      "\n";
  }

  private generateCommands(): string {
    const commands = this.cmd.getCommands(false);
    if (!commands.length) {
      return "";
    }

    const hasTypeDefinitions = !!commands.find((command) =>
      !!command.getArgsDefinition()
    );

    if (hasTypeDefinitions) {
      return this.label("Commands") +
        Table.from([
          ...commands.map((command: Command) => [
            [command.getName(), ...command.getAliases()].map((name) =>
              brightBlue(name)
            ).join(", "),
            highlightArguments(
              command.getArgsDefinition() || "",
              this.options.types,
            ),
            red(bold("-")),
            command.getShortDescription(),
          ]),
        ])
          .indent(this.indent)
          .maxColWidth([60, 60, 1, 80])
          .padding([2, 2, 1, 2])
          .toString() +
        "\n";
    }

    return this.label("Commands") +
      Table.from([
        ...commands.map((command: Command) => [
          [command.getName(), ...command.getAliases()].map((name) =>
            brightBlue(name)
          )
            .join(", "),
          red(bold("-")),
          command.getShortDescription(),
        ]),
      ])
        .maxColWidth([60, 1, 80])
        .padding([2, 1, 2])
        .indent(this.indent)
        .toString() +
      "\n";
  }

  private generateEnvironmentVariables(): string {
    const envVars = this.cmd.getEnvVars(false);
    if (!envVars.length) {
      return "";
    }
    return this.label("Environment variables") +
      Table.from([
        ...envVars.map((envVar: EnvVar) => [
          envVar.names.map((name: string) => brightBlue(name)).join(", "),
          highlightArgumentDetails(
            envVar.details,
            this.options.types,
          ),
          red(bold("-")),
          this.options.long
            ? dedent(envVar.description)
            : envVar.description.trim().split("\n", 1)[0],
          envVar.required ? `(${yellow(`required`)})` : "",
        ]),
      ])
        .padding([2, 2, 1, 2])
        .indent(this.indent)
        .maxColWidth([60, 60, 1, 80, 10])
        .toString() +
      "\n";
  }

  private generateExamples(): string {
    const examples = this.cmd.getExamples();
    if (!examples.length) {
      return "";
    }
    return this.label("Examples") +
      Table.from(examples.map((example: Example) => [
        dim(bold(`${capitalize(example.name)}:`)),
        dedent(example.description),
      ]))
        .padding(1)
        .indent(this.indent)
        .maxColWidth(150)
        .toString() +
      "\n";
  }

  private generateHints(option: Option): string {
    if (!this.options.hints) {
      return "";
    }
    const hints = [];

    option.required && hints.push(yellow(`required`));

    if (typeof option.default !== "undefined") {
      const defaultValue = typeof option.default === "function"
        ? option.default()
        : option.default;

      if (typeof defaultValue !== "undefined") {
        hints.push(
          bold(`Default: `) + inspect(defaultValue, this.options.colors),
        );
      }
    }

    option.depends?.length && hints.push(
      yellow(bold(`Depends: `)) +
        italic(option.depends.map(getFlag).join(", ")),
    );

    option.conflicts?.length && hints.push(
      red(bold(`Conflicts: `)) +
        italic(option.conflicts.map(getFlag).join(", ")),
    );

    const type = this.cmd.getType(option.args[0]?.type)?.handler;
    if (type instanceof Type) {
      const possibleValues = type.values?.(this.cmd, this.cmd.getParent());
      if (possibleValues?.length) {
        hints.push(
          bold(`Values: `) +
            possibleValues.map((value: unknown) =>
              inspect(value, this.options.colors)
            ).join(", "),
        );
      }
    }

    if (hints.length) {
      return `(${hints.join(", ")})`;
    }

    return "";
  }

  private label(label: string) {
    return "\n" + bold(`${label}:`) + "\n\n";
  }
}

function capitalize(string: string): string {
  return string.charAt(0).toUpperCase() + string.slice(1);
}

/**
 * Colorize arguments string.
 * @param argsDefinition Arguments definition: `<color1:string> <color2:string>`
 * @param types Show types.
 */
function highlightArguments(argsDefinition: string, types = true) {
  if (!argsDefinition) {
    return "";
  }

  return parseArgumentsDefinition(argsDefinition, false, true)
    .map((arg: Argument | string) =>
      typeof arg === "string" ? arg : highlightArgumentDetails(arg, types)
    )
    .join(" ");
}

/**
 * Colorize argument string.
 * @param arg Argument details.
 * @param types Show types.
 */
function highlightArgumentDetails(
  arg: Argument,
  types = true,
): string {
  let str = "";

  str += yellow(arg.optional ? "[" : "<");

  let name = "";
  name += arg.name;
  if (arg.variadic) {
    name += "...";
  }
  name = brightMagenta(name);

  str += name;

  if (types) {
    str += yellow(":");
    str += red(arg.type);
    if (arg.list) {
      str += green("[]");
    }
  }

  str += yellow(arg.optional ? "]" : ">");

  return str;
}
