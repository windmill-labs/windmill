import type { Command } from "./command.js";
import type { Type } from "./type.js";
import type { ActionHandler, Argument, CommandResult, CompleteHandler, CompleteOptions, Completion, Description, EnvVar, EnvVarOptions, EnvVarValueHandler, Example, GlobalEnvVarOptions, GlobalOptionOptions, HelpHandler, Option, OptionOptions, OptionValueHandler, TypeDef, TypeOptions, VersionHandler } from "./types.js";
/** @deprecated Use `Argument` instead. */
export type IArgument = Argument;
/** @deprecated Use `GlobalOptionOptions` instead. */
export type ICommandGlobalOption<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = GlobalOptionOptions<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `OptionOptions` instead. */
export type ICommandOption<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = OptionOptions<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `CompleteOptions` instead. */
export type ICompleteOptions = CompleteOptions;
/** @deprecated Use `Completion` instead. */
export type ICompletion<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = Completion<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `EnvVar` instead. */
export type IEnvVar = EnvVar;
/** @deprecated Use `EnvVarOptions` instead. */
export type IEnvVarOptions = EnvVarOptions;
/** @deprecated Use `Example` instead. */
export type IExample = Example;
/** @deprecated Use `GlobalEnvVarOptions` instead. */
export type IGlobalEnvVarOptions = GlobalEnvVarOptions;
/** @deprecated Use `Option` instead. */
export type IOption<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = Option<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `CommandResult` instead. */
export type IParseResult<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = CommandResult<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `TypeDef` instead. */
export type IType = TypeDef;
/** @deprecated Use `TypeOptions` instead. */
export type ITypeOptions = TypeOptions;
/** @deprecated Use `ActionHandler` instead. */
export type IAction<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = ActionHandler<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `CompleteHandler` instead. */
export type ICompleteHandler<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = CompleteHandler<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `Description` instead. */
export type IDescription<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined> = Description<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `EnvVarValueHandler` instead. */
export type IEnvVarValueHandler<TValue = any, TReturn = TValue> = EnvVarValueHandler<TValue, TReturn>;
/** @deprecated Use `OptionValueHandler` instead. */
export type IFlagValueHandler<TValue = any, TReturn = TValue> = OptionValueHandler<TValue, TReturn>;
/** @deprecated Use `HelpHandler` instead. */
export type IHelpHandler<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined, C extends Command<PG, PT, O, A, G, CT, GT, P> = Command<PG, PT, O, A, G, CT, GT, P>> = HelpHandler<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `VersionHandler` instead. */
export type IVersionHandler<O extends Record<string, any> | void = any, A extends Array<unknown> = O extends number ? any : [], G extends Record<string, any> | void = O extends number ? any : void, PG extends Record<string, any> | void = O extends number ? any : void, CT extends Record<string, any> | void = O extends number ? any : void, GT extends Record<string, any> | void = O extends number ? any : void, PT extends Record<string, any> | void = O extends number ? any : void, P extends Command<any> | undefined = O extends number ? any : undefined, C extends Command<PG, PT, O, A, G, CT, GT, P> = Command<PG, PT, O, A, G, CT, GT, P>> = VersionHandler<O, A, G, PG, CT, GT, PT, P>;
/** @deprecated Use `Type.infer` instead. */
export type TypeValue<TType, TDefault = TType> = Type.infer<TType, TDefault>;
//# sourceMappingURL=deprecated.d.ts.map