import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Visitor } from "oxlint-plugin-utilities";

const FLAG_U = "u";
const FLAG_V = "v";

function hasUnicodeFlag(flags: string): boolean {
	return flags.includes(FLAG_U) || flags.includes(FLAG_V);
}

function isIdentifierWithName(node: ESTree.Expression, name: string): node is ESTree.IdentifierReference {
	return node.type === "Identifier" && node.name === name;
}

function getFlagsString(node: ESTree.Node): string | undefined {
	if (node.type !== "Literal" || typeof node.value !== "string") return undefined;
	return node.value;
}

function isNotSpread(node: ESTree.Argument): node is ESTree.Expression {
	return node.type !== "SpreadElement";
}

const requireUnicodeRegex = defineRule({
	create(context): Visitor {
		return {
			CallExpression(node): void {
				if (!isIdentifierWithName(node.callee, "regex")) return;

				if (node.arguments.length < 2) {
					context.report({ messageId: "requireUnicodeFlag", node });
					return;
				}

				const [, flagsNode] = node.arguments;
				if (flagsNode === undefined || !isNotSpread(flagsNode)) return;

				const flags = getFlagsString(flagsNode);
				if (flags === undefined) return;
				if (!hasUnicodeFlag(flags)) {
					context.report({ messageId: "requireUnicodeFlag", node });
				}
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Require the 'u' or 'v' unicode flag on arktype regex() calls.",
		},
		messages: {
			requireUnicodeFlag:
				"Missing the 'u' or 'v' unicode flag on this regex() call. Use the unicode flag to avoid silently creating invalid regex patterns.",
		},
		schema: [],
		type: "problem",
	},
});

export default requireUnicodeRegex;
