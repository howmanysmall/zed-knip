// oxlint-disable typescript/no-unnecessary-condition
import { basename, extname, relative } from "node:path";
import { toPascalCase } from "@oxlint-utilities/casing-utilities";
import { resolveRelativeImport } from "@oxlint-utilities/resolve-import";
import { defineRule } from "oxlint-plugin-utilities";

import type { Visitor } from "oxlint-plugin-utilities";

interface Options {
	readonly allow?: ReadonlyArray<string>;
	readonly maxDepth?: number;
}

function pathSegmentsFromSource(source: string): ReadonlyArray<string> {
	return source.split("/").filter((part) => !part.startsWith("."));
}
function countParentTraversals(pathDiff: string): number {
	return pathDiff.split("/").filter((part) => part === "..").length;
}
function hasAnotherComponentInPath(pathParts: ReadonlyArray<string>): boolean {
	return pathParts.some((part) => part === toPascalCase(part) && !part.includes("."));
}
function isIndexFile(filePath: string): boolean {
	return basename(filePath, extname(filePath)) === "index";
}
function isValidFixtureImport(pathParts: ReadonlyArray<string>): boolean {
	if (!pathParts.includes("fixtures")) return false;

	const fixtureIndex = pathParts.indexOf("fixtures");
	const partsBeforeFixture = pathParts.slice(0, fixtureIndex);

	return !hasAnotherComponentInPath(partsBeforeFixture);
}

const strictComponentBoundaries = defineRule({
	create(context): Visitor {
		const rawOptions = context.options?.[0];
		const { allow = [], maxDepth = 1 } =
			typeof rawOptions === "object" && rawOptions !== null
				? (rawOptions as Options)
				: { allow: [], maxDepth: 1 };

		const allowPatterns = allow.map((pattern) => new RegExp(pattern, "v"));

		return {
			ImportDeclaration(node): void {
				const importSource = node.source.value;
				if (typeof importSource !== "string" || !importSource.startsWith(".")) return;
				if (allowPatterns.some((regexp) => regexp.test(importSource))) return;

				const { filename } = context;
				if (filename === "") return;

				const resolved = resolveRelativeImport(importSource, filename);
				if (!resolved.found) return;

				const pathDifference = relative(filename, resolved.path);
				const pathParts = pathSegmentsFromSource(pathDifference);
				const traversals = countParentTraversals(pathDifference);

				if (
					(traversals > 1 || pathParts.includes("components")) &&
					hasAnotherComponentInPath(pathParts) &&
					pathParts.length > maxDepth &&
					!isIndexFile(pathDifference) &&
					!isValidFixtureImport(pathParts)
				) {
					context.report({
						messageId: "noReachingIntoComponent",
						node,
					});
					return;
				}

				if (
					pathParts.includes("components") &&
					pathParts.length > maxDepth + 1 &&
					!isIndexFile(pathDifference) &&
					!isValidFixtureImport(pathParts)
				) {
					context.report({
						messageId: "noReachingIntoComponent",
						node,
					});
				}
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Prevent module imports between components.",
		},
		messages: {
			noReachingIntoComponent:
				"Do not reach into an individual component's folder for nested modules. Import from the closest shared components folder instead.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					allow: {
						items: { type: "string" },
						type: "array",
					},
					maxDepth: {
						type: "integer",
					},
				},
				type: "object",
			} as const,
		] as const,
		type: "problem",
	},
});

export default strictComponentBoundaries;
