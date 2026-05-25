import { isCallbackFunction, isExportNamedDeclaration } from "@oxlint-utilities/oxc-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ESTree, Fix, Scope, Visitor } from "oxlint-plugin-utilities";

const SCREAMING_SNAKE_CASE = /^[A-Z][A-Z0-9_]*$/u;
const OBJECT_CONSTRUCTOR_PATTERNS: ReadonlyArray<string> = [
	String.raw`^Array\b`,
	String.raw`^Object\b`,
	String.raw`^Map\b`,
	String.raw`^Set\b`,
	String.raw`^WeakMap\b`,
	String.raw`^WeakSet\b`,
];

function collectAllScopes(root: Scope): Array<Scope> {
	const scopes = new Array<Scope>();
	const stack = [root];

	while (stack.length > 0) {
		const current = stack.pop();
		if (current === undefined) break;
		scopes.push(current);
		for (const childScope of current.childScopes) {
			stack.push(childScope);
		}
	}

	return scopes;
}

function isBindingIdentifier(node: ESTree.Node): node is ESTree.BindingIdentifier {
	return node.type === "Identifier";
}

function isVariableDeclarator(node: ESTree.Node): node is ESTree.VariableDeclarator {
	return node.type === "VariableDeclarator";
}

function isVariableDeclaration(node: ESTree.Node): node is ESTree.VariableDeclaration {
	return node.type === "VariableDeclaration";
}

function isFunctionLikeInitializer(node: ESTree.Node): boolean {
	return isCallbackFunction(node) || node.type === "ClassExpression";
}

function isObjectLikeInitializer(
	initializer: ESTree.Expression,
	patterns: ReadonlyArray<RegExp>,
	sourceCode: { getText(node: ESTree.Node): string },
): boolean {
	if (initializer.type === "ArrayExpression" || initializer.type === "ObjectExpression") return true;
	if (initializer.type === "JSXElement" || initializer.type === "JSXFragment") return true;
	if (initializer.type !== "CallExpression" && initializer.type !== "NewExpression") return false;

	const candidateText = sourceCode.getText(initializer.callee);
	return patterns.some((pattern) => pattern.test(candidateText));
}

function isStatementContainer(node: ESTree.Node): node is ESTree.BlockStatement | ESTree.Program {
	return node.type === "Program" || node.type === "BlockStatement";
}

function isSideEffectFreeLiteral(node: ESTree.Expression): boolean {
	return node.type === "Literal";
}

function areAdjacentStatements(first: ESTree.VariableDeclaration, second: ESTree.VariableDeclaration): boolean {
	const { parent } = first;
	if (!isStatementContainer(parent)) return false;

	const { body } = parent;
	for (let index = 0; index < body.length; index += 1) {
		const statement = body[index];
		if (statement === first) {
			const nextStatement = body[index + 1];
			return nextStatement === second;
		}
	}

	return false;
}

function findEnclosingConstDeclarator(node: ESTree.Node): ESTree.VariableDeclarator | undefined {
	let current: ESTree.Node | null | undefined = node.parent;
	let previous: ESTree.Node = node;

	// oxlint-disable-next-line typescript/no-unnecessary-condition
	while (current !== undefined && current !== null) {
		if (isVariableDeclarator(current) && current.init === previous) return current;

		previous = current;
		current = current.parent;
	}

	return undefined;
}

const noUselessConstants = defineRule({
	create(context): Visitor {
		const { sourceCode } = context;
		const [rawOptions] = context.options;
		// oxlint-disable-next-line typescript/no-unnecessary-condition
		const ignoreCallPatterns = rawOptions?.ignoreCallPatterns ?? OBJECT_CONSTRUCTOR_PATTERNS;
		const ignoredCallPatternMatchers = ignoreCallPatterns.map((pattern) => new RegExp(pattern, "u"));

		function hasAttachedComments(node: ESTree.Node): boolean {
			if (sourceCode.getCommentsInside(node).length > 0) return true;

			const nodeStartLine = node.loc.start.line;
			const nodeEndLine = node.loc.end.line;

			for (const comment of sourceCode.getCommentsBefore(node)) {
				const commentEndLine = comment.loc.end.line;
				if (commentEndLine === nodeStartLine || commentEndLine === nodeStartLine - 1) return true;
			}

			for (const comment of sourceCode.getCommentsAfter(node)) {
				const commentStartLine = comment.loc.start.line;
				if (commentStartLine === nodeEndLine || commentStartLine === nodeEndLine + 1) return true;
			}

			return false;
		}

		return {
			"Program:exit": function (programNode): void {
				const programScope = sourceCode.getScope(programNode);
				const allScopes = collectAllScopes(programScope);

				for (const scope of allScopes) {
					if (scope.type === "global") continue;

					for (const scopeVariable of scope.variables) {
						const { name } = scopeVariable;
						if (!SCREAMING_SNAKE_CASE.test(name)) continue;

						const [variableDefinition] = scopeVariable.defs;
						if (variableDefinition?.type !== "Variable") continue;

						const declaratorNode = variableDefinition.node;
						if (!isVariableDeclarator(declaratorNode) || !isBindingIdentifier(declaratorNode.id)) continue;
						if (declaratorNode.init === null) continue;

						const declarationNode = variableDefinition.parent;
						if (declarationNode === null || !isVariableDeclaration(declarationNode)) continue;
						if (declarationNode.kind !== "const" || declarationNode.declarations.length !== 1) continue;

						const declarationParentNode = declarationNode.parent;
						if (isExportNamedDeclaration(declarationParentNode)) continue;

						const initializer = declaratorNode.init;
						if (isFunctionLikeInitializer(initializer)) continue;
						if (isObjectLikeInitializer(initializer, ignoredCallPatternMatchers, sourceCode)) continue;

						let readOnlyReference: Scope["references"][number] | undefined;
						let readOnlyCount = 0;
						for (const scopeReference of scopeVariable.references) {
							if (scopeReference.isReadOnly()) {
								readOnlyCount += 1;
								readOnlyReference = scopeReference;
							}
						}
						if (
							readOnlyCount !== 1 ||
							readOnlyReference === undefined ||
							readOnlyReference.from !== scope ||
							sourceCode.getScope(readOnlyReference.identifier) !== scope
						) {
							continue;
						}

						const referenceIdentifier = readOnlyReference.identifier;
						if (!isBindingIdentifier(referenceIdentifier)) continue;

						const enclosingDeclarator = findEnclosingConstDeclarator(referenceIdentifier);
						if (enclosingDeclarator === undefined) continue;

						const enclosingDeclaration = enclosingDeclarator.parent;
						if (!isVariableDeclaration(enclosingDeclaration) || enclosingDeclaration.kind !== "const") {
							continue;
						}

						const canFix =
							(areAdjacentStatements(declarationNode, enclosingDeclaration) ||
								isSideEffectFreeLiteral(initializer)) &&
							!hasAttachedComments(declarationNode);

						if (!canFix) {
							context.report({
								data: { name },
								messageId: "uselessConstantNoFix",
								node: declaratorNode.id,
							});
							continue;
						}

						const initializerText = sourceCode.getText(initializer);

						context.report({
							data: { name },
							fix(fixer): Array<Fix> {
								const fixes = new Array<Fix>();

								fixes.push(fixer.replaceText(referenceIdentifier, initializerText));

								let [start] = declarationNode.range;
								while (start > 0) {
									const previousCharacter = sourceCode.text[start - 1];
									if (previousCharacter === " " || previousCharacter === "\t") {
										start -= 1;
										continue;
									}
									break;
								}

								// oxlint-disable-next-line prefer-destructuring
								let end = declarationNode.range[1];
								while (end < sourceCode.text.length) {
									const nextCharacter = sourceCode.text[end];
									if (nextCharacter === "\n" || nextCharacter === "\r") {
										end += 1;
										continue;
									}
									break;
								}

								fixes.push(fixer.removeRange([start, end]));
								return fixes;
							},
							messageId: "uselessConstant",
							node: declaratorNode.id,
						});
					}
				}
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Disallow constants that do not add value.",
			recommended: true,
		},
		fixable: "code",
		messages: {
			uselessConstant:
				"Constant '{{name}}' is only referenced once in the same scope. Inline it directly, or move it to a higher scope if reference stability is needed.",
			uselessConstantNoFix:
				"Constant '{{name}}' is only referenced once in the same scope. It cannot be auto-inlined because the declaration is not adjacent to its usage or has attached comments. Inline it manually, or move it to a higher scope if reference stability is needed.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					ignoreCallPatterns: {
						default: [...OBJECT_CONSTRUCTOR_PATTERNS],
						items: { type: "string" },
						type: "array",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default noUselessConstants;
