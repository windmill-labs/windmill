declare var SSOOIDCClient: {
  new (__0_0: any): {
    destroy(): void;
  };
};
import { Command as $Command } from "@smithy/smithy-client";
import { HttpRequest as __HttpRequest } from "@smithy/protocol-http";
declare var AccessDeniedException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var AuthorizationPendingException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var ExpiredTokenException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var InternalServerException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var InvalidClientException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var InvalidRequestException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var InvalidScopeException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var SlowDownException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var UnauthorizedClientException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare var UnsupportedGrantTypeException: {
  new (opts: any): {
    readonly $fault: "server" | "client";
    $response?: import("@smithy/types").HttpResponse | undefined;
    $retryable?: import("@smithy/types").RetryableTrait | undefined;
    $metadata: import("@smithy/types").ResponseMetadata;
    name: string;
    message: string;
    stack?: string | undefined;
  };
  captureStackTrace(
    targetObject: object,
    constructorOpt?: Function | undefined
  ): void;
  prepareStackTrace?:
    | ((err: Error, stackTraces: NodeJS.CallSite[]) => any)
    | undefined;
  stackTraceLimit: number;
};
declare class CreateTokenCommand extends $Command {
  constructor(input: any);
  static getEndpointParameterInstructions(): {
    UseFIPS: {
      type: string;
      name: string;
    };
    Endpoint: {
      type: string;
      name: string;
    };
    Region: {
      type: string;
      name: string;
    };
    UseDualStack: {
      type: string;
      name: string;
    };
  };
  resolveMiddleware(clientStack: any, configuration: any, options: any): any;
  serialize(input: any, context: any): Promise<__HttpRequest>;
  deserialize(output: any, context: any): Promise<any>;
}
export {
  AccessDeniedException,
  AuthorizationPendingException,
  CreateTokenCommand,
  ExpiredTokenException,
  InternalServerException,
  InvalidClientException,
  InvalidRequestException,
  InvalidScopeException,
  SSOOIDCClient,
  SlowDownException,
  UnauthorizedClientException,
  UnsupportedGrantTypeException,
};
