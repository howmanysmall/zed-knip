import { defineRule } from "oxlint-plugin-utilities";

import type { Context, ESTree, Visitor } from "oxlint-plugin-utilities";

type MessageId = "missingAsyncSuffix";

function hasAsyncSuffix(name: string): boolean {
	return name.endsWith("Async");
}

function reportIdentifierIfMissingAsyncSuffix(
	context: Context<readonly [], MessageId>,
	node: ESTree.IdentifierName,
): void {
	if (hasAsyncSuffix(node.name)) return;
	context.report({ messageId: "missingAsyncSuffix", node });
}

function reportVariableDeclarator(context: Context<readonly [], MessageId>, node: ESTree.VariableDeclarator): void {
	if (node.id.type !== "Identifier") return;
	if (node.init?.type !== "ArrowFunctionExpression" && node.init?.type !== "FunctionExpression") return;
	if (!node.init.async) return;
	reportIdentifierIfMissingAsyncSuffix(context, node.id);
}

function reportClassMethod(context: Context<readonly [], MessageId>, node: ESTree.MethodDefinition): void {
	if (!node.value.async || node.key.type !== "Identifier") return;
	reportIdentifierIfMissingAsyncSuffix(context, node.key);
}

const requireAsyncSuffix = defineRule({
	create(context): Visitor {
		return {
			FunctionDeclaration(node): void {
				if (!node.async || node.id === null) return;
				reportIdentifierIfMissingAsyncSuffix(context, node.id);
			},
			MethodDefinition(node): void {
				reportClassMethod(context, node);
			},
			Property(node): void {
				if (!node.method || node.value.type !== "FunctionExpression" || !node.value.async) return;
				if (node.key.type !== "Identifier") return;
				reportIdentifierIfMissingAsyncSuffix(context, node.key);
			},
			PropertyDefinition(node): void {
				if (node.value?.type !== "ArrowFunctionExpression" && node.value?.type !== "FunctionExpression") return;
				if (!node.value.async || node.key.type !== "Identifier") return;
				reportIdentifierIfMissingAsyncSuffix(context, node.key);
			},
			VariableDeclarator(node): void {
				reportVariableDeclarator(context, node);
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Require async function names to end with Async.",
		},
		messages: {
			missingAsyncSuffix: "Async functions must have names that end with Async.",
		},
		schema: [] as const,
		type: "problem",
	},
});

export default requireAsyncSuffix;
