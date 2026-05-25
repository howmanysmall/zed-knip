import { isRecord } from "@oxlint-utilities/type-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Scope, Visitor } from "oxlint-plugin-utilities";

interface TrackedInstantiation {
	readonly className: string;
	readonly importSource: string;
}

function normalizeConfig(options: unknown): ReadonlyMap<string, string> {
	if (!isRecord(options) || !("classes" in options) || !isRecord(options.classes)) return new Map();
	const { classes } = options;
	const result = new Map<string, string>();
	for (const [key, value] of Object.entries(classes)) {
		if (typeof value !== "string") continue;
		result.set(key, value);
	}
	return result;
}

function isModuleScope(scope: null | Scope): boolean {
	return scope?.type === "module" || scope?.type === "global";
}

function getImportedClassName(specifier: ESTree.ImportDeclaration["specifiers"][number]): string | undefined {
	if (specifier.type === "ImportDefaultSpecifier") return specifier.local.name;

	if (specifier.type !== "ImportSpecifier") return undefined;
	if (specifier.imported.type === "Identifier") return specifier.imported.name;
	if (typeof specifier.imported.value === "string") return specifier.imported.value;
	return undefined;
}

function getTrackedInstantiation(
	node: ESTree.NewExpression,
	localBindings: ReadonlyMap<string, string>,
	trackedClasses: ReadonlyMap<string, string>,
): TrackedInstantiation | undefined {
	if (node.callee.type === "Identifier") {
		const className = localBindings.get(node.callee.name);
		if (className === undefined) return undefined;

		const importSource = trackedClasses.get(className);
		if (importSource === undefined) return undefined;

		return { className, importSource };
	}

	if (node.callee.type === "MemberExpression" && node.callee.property.type === "Identifier") {
		const className = node.callee.property.name;
		const importSource = trackedClasses.get(className);
		if (importSource === undefined) return undefined;

		return { className, importSource };
	}

	return undefined;
}

function collectTrackedBindings(
	node: ESTree.ImportDeclaration,
	trackedClasses: ReadonlyMap<string, string>,
	localBindings: Map<string, string>,
): void {
	if (typeof node.source.value !== "string") return;

	const importSource = node.source.value;

	for (const [className, trackedImportSource] of trackedClasses) {
		if (trackedImportSource !== importSource) continue;

		for (const specifier of node.specifiers) {
			const importedClassName = getImportedClassName(specifier);
			if (importedClassName !== className) continue;
			localBindings.set(specifier.local.name, className);
		}
	}
}

const requireModuleLevelInstantiation = defineRule({
	create(context): Visitor {
		const { sourceCode } = context;
		const trackedClasses = normalizeConfig(context.options[0]);
		if (trackedClasses.size === 0) return {} satisfies Visitor;

		const localBindings = new Map<string, string>();

		return {
			ImportDeclaration(node): void {
				collectTrackedBindings(node, trackedClasses, localBindings);
			},
			NewExpression(node): void {
				const trackedInstantiation = getTrackedInstantiation(node, localBindings, trackedClasses);
				if (trackedInstantiation === undefined) return;

				if (isModuleScope(sourceCode.getScope(node))) return;

				context.report({
					data: {
						className: trackedInstantiation.className,
						importSource: trackedInstantiation.importSource,
					},
					messageId: "mustBeModuleLevel",
					node,
				});
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Require configured classes to be instantiated at module level only.",
		},
		messages: {
			mustBeModuleLevel: "'{{className}}' from '{{importSource}}' must be instantiated at module level only.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					classes: {
						additionalProperties: { type: "string" },
						type: "object",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default requireModuleLevelInstantiation;
