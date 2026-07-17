import type { Type, StructType, InterfaceType, CallableSignature } from "./types.js";
import type { ErrorRegistry } from "./errors.js";

export type ReceiverContext = {
  structName: string;
  fields: Record<string, Type>;
};

export type TypeEnv = {
  variables: Record<string, Type>;
  structs: Record<string, StructType>;
  interfaces: Record<string, InterfaceType>;
  functions: Record<string, CallableSignature>;
  receiver?: ReceiverContext;
  errorRegistry?: ErrorRegistry;
};

export function createEnv(): TypeEnv {
  return { variables: {}, structs: {}, interfaces: {}, functions: {} };
}
