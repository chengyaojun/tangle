import type { TangleDirective, SourceSpan } from "../model.js";
import type { Type } from "./types.js";
export type ErrorVariant = {
    name: string;
    fields: Record<string, Type>;
    span: SourceSpan;
};
export declare class ErrorRegistry {
    private variants;
    register(name: string, fields: Record<string, Type>, span?: SourceSpan): void;
    lookup(name: string): ErrorVariant | undefined;
    isError(name: string): boolean;
    collectFromDirectives(directives: TangleDirective[]): void;
    allVariants(): ErrorVariant[];
}
