import type { Type, StructType, InterfaceType } from "./types.js";
import type { ErrorRegistry } from "./errors.js";
export type ReceiverContext = {
    structName: string;
    fields: Record<string, Type>;
};
export type TypeEnv = {
    variables: Record<string, Type>;
    structs: Record<string, StructType>;
    interfaces: Record<string, InterfaceType>;
    receiver?: ReceiverContext;
    errorRegistry?: ErrorRegistry;
};
export declare function createEnv(): TypeEnv;
