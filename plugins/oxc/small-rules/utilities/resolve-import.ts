import { dirname } from "node:path";
import { ResolverFactory } from "oxc-resolver";

type ResolveResult =
	| { readonly found: false }
	| {
			readonly found: true;
			readonly path: string;
	  };

const resolver = new ResolverFactory({
	extensions: [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".json", ".node"],
});

const NOT_FOUND: ResolveResult = { found: false };

/**
 * Resolves a relative import to an absolute path.
 *
 * @param importSource - The import specifier (e.g., "./foo/bar").
 * @param sourceFile - The absolute path of the file containing the import.
 * @returns Resolution result with path if found.
 */
export function resolveRelativeImport(importSource: string, sourceFile: string): ResolveResult {
	if (!importSource.startsWith(".")) return NOT_FOUND;

	const { path } = resolver.sync(dirname(sourceFile), importSource);
	return path === undefined || path === "" ? NOT_FOUND : { found: true, path };
}
