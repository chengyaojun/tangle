export type Type = PrimitiveType | StructType | SumType_ | GenericTypeInstance | FunctionType_ | InterfaceType | TypeVariable;
export type PrimitiveType = {
    kind: "primitive";
    name: "String" | "Int" | "Bool";
};
export type StructType = {
    kind: "struct";
    name: string;
    fields: Record<string, Type>;
    methods: Record<string, CallableSignature>;
};
export type SumType_ = {
    kind: "sum";
    variants: Type[];
};
export type GenericTypeInstance = {
    kind: "genericInstance";
    base: string;
    args: Type[];
};
export type FunctionType_ = {
    kind: "function";
    params: Type[];
    returns: Type;
};
export type InterfaceType = {
    kind: "interface";
    name: string;
    methods: Record<string, CallableSignature>;
};
export type TypeVariable = {
    kind: "var";
    id: number;
};
export type CallableSignature = {
    params: {
        name: string;
        type: Type;
    }[];
    returns: Type;
};
export declare function typesEqual(a: Type, b: Type): boolean;
export declare function isSubtype(value: Type, target: Type): boolean;
