#!/usr/bin/env node
import { readFileSync, existsSync } from "fs";
import { resolve } from "path";
import { compile } from "../pipeline.js";
function printDiagnostics(diags) {
    for (const d of diags) {
        console.error(`${d.code}: ${d.message}`);
        console.error(`  --> ${d.span.file}:${d.span.startLine}:${d.span.startColumn}`);
    }
}
async function main() {
    const args = process.argv.slice(2);
    if (args.length === 0) {
        console.log("Tangle v0.1.0");
        console.log("Usage: tangle run <file.md>");
        console.log("       tangle test [--filter <pattern>]");
        process.exit(0);
    }
    const command = args[0];
    if (command === "run") {
        const filePath = args[1];
        if (!filePath) {
            console.error("Error: No file specified. Usage: tangle run <file.md>");
            process.exit(1);
        }
        const absPath = resolve(filePath);
        if (!existsSync(absPath)) {
            console.error(`Error: File not found: ${absPath}`);
            process.exit(1);
        }
        const source = readFileSync(absPath, "utf-8");
        const result = compile(source, filePath);
        if (result.diagnostics.some(d => d.code.startsWith("TANGLE_TYPE_") || d.code.startsWith("TANGLE_PARSE_"))) {
            console.error("Compilation failed with errors:");
            printDiagnostics(result.diagnostics.filter(d => d.code.startsWith("TANGLE_TYPE_") || d.code.startsWith("TANGLE_PARSE_")));
            process.exit(1);
        }
        if (result.diagnostics.length > 0) {
            console.error("Warnings:");
            printDiagnostics(result.diagnostics);
        }
        console.log(result.js);
    }
    else if (command === "test") {
        console.log("Tangle test — collecting @test directives...");
        // For now, test just validates compilation
        const filterIdx = args.indexOf("--filter");
        const pattern = filterIdx >= 0 ? args[filterIdx + 1] : undefined;
        console.log(pattern ? `Filter: ${pattern}` : "Running all tests");
        console.log("(Test runner: full implementation in progress)");
    }
    else {
        console.error(`Unknown command: ${command}`);
        console.log("Usage: tangle run <file.md>");
        process.exit(1);
    }
}
main().catch(err => {
    console.error("Fatal:", err.message);
    process.exit(1);
});
