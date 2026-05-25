// oxlint-disable typescript/no-unnecessary-condition
// oxlint-disable small-rules/prevent-abbreviations
import { defineRule } from "oxlint-plugin-utilities";

import type { Comment, ESTree, Visitor } from "oxlint-plugin-utilities";

type AnyImportSpecifier = ESTree.ImportDefaultSpecifier | ESTree.ImportNamespaceSpecifier | ESTree.ImportSpecifier;

interface ImportInfo {
	readonly identifierName: string;
	readonly parent: ESTree.ImportDeclaration;
	readonly specifier: AnyImportSpecifier;
}

const JSDOC_PATTERN = new RegExp(
	String.raw`(?:@(?:link|linkcode|linkplain|see)\s+\{?\w+\b\}?)|` +
		String.raw`(?:\{@(?:link|linkcode|linkplain|see)\s+\w+\b\})|` +
		String.raw`(?:[@{](?:type|typedef|param|returns?|template|augments|extends|implements)\s+[^}]*\b\w+\b)`,
	"u",
);

const JSDOC_IDENTIFIER_PATTERN = new RegExp(
	String.raw`(?:@(?:link|linkcode|linkplain|see)\s+\{?(\w+)\b\}?)|` +
		String.raw`(?:\{@(?:link|linkcode|linkplain|see)\s+(\w+)\b\})|` +
		String.raw`(?:[@{](?:type|typedef|param|returns?|template|augments|extends|implements)\s+[^}]*\b(\w+)\b)`,
	"gu",
);

function isImportSpecifier(node: ESTree.Node): node is AnyImportSpecifier {
	return (
		node.type === "ImportDefaultSpecifier" ||
		node.type === "ImportNamespaceSpecifier" ||
		node.type === "ImportSpecifier"
	);
}

function collectJsDocIdentifiers(comments: ReadonlyArray<Comment>): Set<string> {
	const identifiers = new Set<string>();
	for (const comment of comments) {
		if (comment.type !== "Block" || !JSDOC_PATTERN.test(comment.value)) continue;
		JSDOC_IDENTIFIER_PATTERN.lastIndex = 0;
		for (const match of comment.value.matchAll(JSDOC_IDENTIFIER_PATTERN)) {
			const identifier = match[1] ?? match[2] ?? match[3];
			if (identifier !== undefined) identifiers.add(identifier);
		}
	}
	return identifiers;
}

const noUnusedImports = defineRule({
	create(context): Visitor {
		const { sourceCode } = context;
		const checkJsDoc = context.options[0]?.checkJSDoc ?? true;
		const jsdocIdentifiers = checkJsDoc ? collectJsDocIdentifiers(sourceCode.getAllComments()) : new Set<string>();

		const imports = new Array<ImportInfo>();
		let scopeReference: ESTree.ImportDeclaration | undefined;

		return {
			ImportDeclaration(node): void {
				scopeReference ??= node;
				for (const specifier of node.specifiers) {
					if (!isImportSpecifier(specifier)) continue;
					imports.push({
						identifierName: specifier.local.name,
						parent: node,
						specifier,
					});
				}
			},

			"Program:exit": function (): void {
				if (scopeReference === undefined) return;
				const moduleScope = sourceCode.getScope(scopeReference);

				for (const { identifierName, parent, specifier: specifierNode } of imports) {
					const variable = moduleScope.set.get(identifierName);
					if (variable !== undefined && variable.references.length > 0) continue;
					if (checkJsDoc && jsdocIdentifiers.has(identifierName)) continue;

					context.report({
						data: { identifierName },
						fix(fixer) {
							if (parent.specifiers.length === 1) return fixer.remove(parent);
							const nextToken = sourceCode.getTokenAfter(specifierNode);
							const isFirstSpecifier = parent.specifiers[0] === specifierNode;
							if (isFirstSpecifier && nextToken?.value === ",") {
								const previousToken = sourceCode.getTokenBefore(specifierNode);
								if (previousToken !== null) {
									return [
										fixer.removeRange([previousToken.range[1], specifierNode.range[0]]),
										fixer.remove(specifierNode),
										fixer.remove(nextToken),
									];
								}
							}
							if (nextToken?.value === ",") {
								return fixer.removeRange([specifierNode.range[0], nextToken.range[1]]);
							}
							const previousToken = sourceCode.getTokenBefore(specifierNode);
							if (previousToken?.value === ",") {
								return fixer.removeRange([previousToken.range[0], specifierNode.range[1]]);
							}
							return fixer.remove(specifierNode);
						},
						messageId: "unusedImport",
						node: specifierNode,
					});
				}
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Disallow unused imports",
		},
		fixable: "code",
		messages: {
			unusedImport: "Import '{{identifierName}}' is defined but never used.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					checkJSDoc: {
						default: true,
						description: "Check if imports are referenced in JSDoc comments",
						type: "boolean",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default noUnusedImports;
