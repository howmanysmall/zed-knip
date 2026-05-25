// oxlint-disable typescript/no-unnecessary-condition
import { getMemberPropertyName, hasShadowedBinding, unwrapExpression } from "@oxlint-utilities/ast-utilities";
import {
	isArrayExpression,
	isArrowFunctionExpression,
	isAssignmentPattern,
	isBinaryExpression,
	isCallExpression,
	isClassNode,
	isConditionalExpression,
	isExpressionStatement,
	isFunctionDeclarationNode,
	isLiteralNode,
	isLogicalExpression,
	isMemberExpression,
	isNewExpression,
	isObjectExpression,
	isPropertyDefinitionNode,
	isSequenceExpression,
	isTemplateLiteral,
	isThisExpression,
	isTsAsExpression,
	isTsTypeAssertion,
	isUnaryExpression,
	isVariableDeclarationNode,
	isVariableDeclarator,
} from "@oxlint-utilities/oxc-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { BindingName } from "@oxlint-types/missing-types";
import type { FixReturn } from "@oxlint-utilities/oxc-utilities";
import type { Environment } from "@oxlint-utilities/react-utilities";
import type { ESTree, Fix, Fixer, SourceCode, Visitor } from "oxlint-plugin-utilities";

interface NoArrayConstructorElementsOptions {
	readonly environment?: Environment;
	readonly requireExplicitGenericOnNewArray?: boolean;
}

type ProgramStatement = ESTree.ModuleDeclaration | ESTree.Statement;

const DEFAULT_OPTIONS: Required<NoArrayConstructorElementsOptions> = {
	environment: "standard",
	requireExplicitGenericOnNewArray: true,
};

function isIdentifier(
	node: BindingName | ESTree.Expression | ESTree.TSTypeName,
): node is ESTree.BindingIdentifier | ESTree.IdentifierReference {
	return node.type === "Identifier";
}

function isGlobalArrayConstructor(sourceCode: SourceCode, node: ESTree.NewExpression): boolean {
	const callee = unwrapExpression(node.callee);
	if (!isIdentifier(callee) || callee.name !== "Array") return false;
	return !hasShadowedBinding(sourceCode, callee, "Array");
}

function extractElementTypeFromArrayAnnotation(typeNode: ESTree.TSType, sourceCode: SourceCode): string | undefined {
	if (typeNode.type !== "TSTypeReference") return undefined;
	if (!isIdentifier(typeNode.typeName)) return undefined;
	if (typeNode.typeName.name !== "Array" && typeNode.typeName.name !== "ReadonlyArray") return undefined;
	if (typeNode.typeArguments?.params.length !== 1) return undefined;

	const [elementType] = typeNode.typeArguments.params;
	return elementType === undefined ? undefined : sourceCode.getText(elementType);
}

const IS_ANNOTATION = /:\s*(Array<.+>|ReadonlyArray<.+>)\s*=/u;

function hasArrayAnnotationInAssignmentPatternText(assignmentText: string): boolean {
	return IS_ANNOTATION.exec(assignmentText) !== null;
}

function getBindingTypeAnnotation(bindingName: BindingName): ESTree.TSTypeAnnotation | undefined {
	if (isIdentifier(bindingName)) return bindingName.typeAnnotation ?? undefined;
	if (bindingName.type === "ArrayPattern") return bindingName.typeAnnotation ?? undefined;
	if (bindingName.type === "ObjectPattern") return bindingName.typeAnnotation ?? undefined;
	return undefined;
}

function hasContextualArrayAnnotation(node: ESTree.NewExpression, sourceCode: SourceCode): boolean {
	const { parent } = node;

	if (isVariableDeclarator(parent) && parent.init === node) {
		const typeAnnotation = getBindingTypeAnnotation(parent.id);
		if (typeAnnotation === undefined) return false;
		return extractElementTypeFromArrayAnnotation(typeAnnotation.typeAnnotation, sourceCode) !== undefined;
	}

	if (isAssignmentPattern(parent) && parent.right === node) {
		return hasArrayAnnotationInAssignmentPatternText(sourceCode.getText(parent));
	}

	if (
		isPropertyDefinitionNode(parent) &&
		parent.value === node &&
		parent.typeAnnotation !== undefined &&
		parent.typeAnnotation !== null
	) {
		return extractElementTypeFromArrayAnnotation(parent.typeAnnotation.typeAnnotation, sourceCode) !== undefined;
	}

	if (isTsAsExpression(parent) && parent.expression === node) {
		return extractElementTypeFromArrayAnnotation(parent.typeAnnotation, sourceCode) !== undefined;
	}

	if (isTsTypeAssertion(parent) && parent.expression === node) {
		return extractElementTypeFromArrayAnnotation(parent.typeAnnotation, sourceCode) !== undefined;
	}

	return false;
}

function isReadonlyArrayAnnotation(typeAnnotation: ESTree.TSTypeAnnotation | undefined): boolean {
	if (typeAnnotation === undefined) return false;
	const { typeAnnotation: annotationType } = typeAnnotation;
	if (annotationType.type !== "TSTypeReference" || !isIdentifier(annotationType.typeName)) return false;
	return annotationType.typeName.name === "ReadonlyArray";
}

function isDefinitelyNonNumericExpression(expression: ESTree.Expression): boolean {
	const unwrapped = unwrapExpression(expression);
	if (isLiteralNode(unwrapped) && "value" in unwrapped) return typeof unwrapped.value !== "number";

	if (
		isArrayExpression(unwrapped) ||
		isObjectExpression(unwrapped) ||
		isArrowFunctionExpression(unwrapped) ||
		isFunctionDeclarationNode(unwrapped) ||
		isClassNode(unwrapped)
	) {
		return true;
	}

	if (isTemplateLiteral(unwrapped)) return unwrapped.expressions.length === 0;
	if (isUnaryExpression(unwrapped)) {
		return unwrapped.operator === "void" || (unwrapped.operator === "typeof" && !unwrapped.prefix);
	}

	return false;
}

function isComputedPropertyKeySafe(key: ESTree.PropertyKey): boolean {
	if (key.type === "Identifier") return true;
	if (key.type === "PrivateIdentifier") return true;
	return isExpressionSideEffectSafe(key);
}

function isExpressionSideEffectSafe(expression: ESTree.Expression): boolean {
	const unwrapped = unwrapExpression(expression);

	if (isIdentifier(unwrapped)) return true;
	if (isThisExpression(unwrapped)) return true;

	if (isMemberExpression(unwrapped)) {
		if (unwrapped.optional || unwrapped.object.type === "Super") return false;
		if (!isExpressionSideEffectSafe(unwrapped.object)) return false;
		if (!unwrapped.computed) return true;
		return isExpressionSideEffectSafe(unwrapped.property);
	}

	if (isUnaryExpression(unwrapped)) {
		if (unwrapped.operator === "delete") return false;
		return isExpressionSideEffectSafe(unwrapped.argument);
	}

	if (isBinaryExpression(unwrapped) || isLogicalExpression(unwrapped)) {
		return isExpressionSideEffectSafe(unwrapped.left) && isExpressionSideEffectSafe(unwrapped.right);
	}

	if (isConditionalExpression(unwrapped)) {
		return (
			isExpressionSideEffectSafe(unwrapped.test) &&
			isExpressionSideEffectSafe(unwrapped.consequent) &&
			isExpressionSideEffectSafe(unwrapped.alternate)
		);
	}

	if (isTemplateLiteral(unwrapped)) {
		for (const part of unwrapped.expressions) if (!isExpressionSideEffectSafe(part)) return false;
		return true;
	}

	if (isArrayExpression(unwrapped)) {
		for (const element of unwrapped.elements) {
			if (element === null) continue;
			if (element.type === "SpreadElement") return false;
			if (!isExpressionSideEffectSafe(element)) return false;
		}
		return true;
	}

	if (isObjectExpression(unwrapped)) {
		for (const property of unwrapped.properties) {
			if (property.type === "SpreadElement" || property.kind !== "init" || property.method) return false;
			if (property.computed && !isComputedPropertyKeySafe(property.key)) return false;
			if (!isExpressionSideEffectSafe(property.value)) return false;
		}
		return true;
	}

	if (isSequenceExpression(unwrapped)) {
		for (const value of unwrapped.expressions) if (!isExpressionSideEffectSafe(value)) return false;
		return true;
	}

	return isLiteralNode(unwrapped);
}

function getPushCallForIdentifier(
	expression: ESTree.Expression,
	identifierName: string,
): ESTree.CallExpression | undefined {
	const unwrapped = unwrapExpression(expression);
	if (!isCallExpression(unwrapped) || unwrapped.optional) return undefined;
	if (!isMemberExpression(unwrapped.callee) || unwrapped.callee.optional) return undefined;
	if (!isIdentifier(unwrapped.callee.object) || unwrapped.callee.object.name !== identifierName) {
		return undefined;
	}

	return getMemberPropertyName(unwrapped.callee) === "push" ? unwrapped : undefined;
}

function containsLaterPushCall(
	statements: ReadonlyArray<ProgramStatement>,
	startIndex: number,
	identifierName: string,
): boolean {
	for (let index = startIndex; index < statements.length; index += 1) {
		const statement = statements[index];
		if (statement === undefined || !isExpressionStatement(statement)) continue;
		if (getPushCallForIdentifier(statement.expression, identifierName) !== undefined) return true;
	}

	return false;
}

function buildArrayLiteralFromArguments(argumentsList: ReadonlyArray<ESTree.Argument>, sourceCode: SourceCode): string {
	const parts = new Array<string>();
	let size = 0;

	for (const argument of argumentsList) {
		if (argument.type === "SpreadElement") {
			parts[size++] = `...${sourceCode.getText(argument.argument)}`;
			continue;
		}

		parts[size++] = sourceCode.getText(argument);
	}

	return `[${parts.join(", ")}]`;
}

function createCollapseFixes(
	fixer: Fixer,
	sourceCode: SourceCode,
	declarator: ESTree.VariableDeclarator,
	pushStatements: ReadonlyArray<ESTree.ExpressionStatement>,
	arrayLiteralText: string,
): Array<Fix> {
	if (declarator.init === null || pushStatements.length === 0) return [];

	const [firstPushStatement] = pushStatements;
	const lastPushStatement = pushStatements.at(-1);
	if (firstPushStatement === undefined || lastPushStatement === undefined) return [];

	let [collapseStart] = firstPushStatement.range;
	while (collapseStart > 0) {
		const previousCharacter = sourceCode.text[collapseStart - 1];
		if (previousCharacter === " " || previousCharacter === "\t") {
			collapseStart -= 1;
			continue;
		}

		if (previousCharacter === "\n") collapseStart -= 1;
		break;
	}

	return [
		fixer.replaceText(declarator.init, arrayLiteralText),
		fixer.removeRange([collapseStart, lastPushStatement.range[1]]),
	];
}

const noArrayConstructorElements = defineRule({
	create(context): Visitor {
		const rawOptions = context.options?.[0];
		const options: Required<NoArrayConstructorElementsOptions> =
			typeof rawOptions === "object" && rawOptions !== null
				? { ...DEFAULT_OPTIONS, ...(rawOptions as Partial<NoArrayConstructorElementsOptions>) }
				: { ...DEFAULT_OPTIONS };
		const { sourceCode } = context;

		function inspectPushCollapse(statements: ReadonlyArray<ProgramStatement>): void {
			for (let index = 0; index < statements.length; index += 1) {
				const statement = statements[index];
				if (statement === undefined || !isVariableDeclarationNode(statement)) continue;
				if (statement.kind !== "const" && statement.kind !== "let") continue;
				if (statement.declarations.length !== 1) continue;

				const [declarator] = statement.declarations;
				if (declarator === undefined || !isIdentifier(declarator.id)) continue;
				if (declarator.init === null || !isNewExpression(declarator.init)) continue;
				if (!isGlobalArrayConstructor(sourceCode, declarator.init)) continue;
				if (declarator.init.arguments.length > 0) continue;
				if (isReadonlyArrayAnnotation(getBindingTypeAnnotation(declarator.id))) continue;
				const arrayIdentifierName = declarator.id.name;

				const pushStatements = new Array<ESTree.ExpressionStatement>();
				const collapsedArgumentParts = new Array<string>();
				let hasSpreadArgument = false;
				let scanIndex = index + 1;

				while (scanIndex < statements.length) {
					const nextStatement = statements[scanIndex];
					if (nextStatement === undefined || !isExpressionStatement(nextStatement)) break;

					const pushCall = getPushCallForIdentifier(nextStatement.expression, arrayIdentifierName);
					if (pushCall === undefined || pushCall.arguments.length === 0) break;

					pushStatements.push(nextStatement);
					for (const argument of pushCall.arguments) {
						if (argument.type === "SpreadElement") {
							hasSpreadArgument = true;
							collapsedArgumentParts.push(`...${sourceCode.getText(argument.argument)}`);
							continue;
						}

						collapsedArgumentParts.push(sourceCode.getText(argument));
					}

					scanIndex += 1;
				}

				if (pushStatements.length === 0 || containsLaterPushCall(statements, scanIndex, arrayIdentifierName)) {
					continue;
				}

				const literalText = `[${collapsedArgumentParts.join(", ")}]`;

				const hasUnsafeArgument =
					hasSpreadArgument ||
					pushStatements.some((pushStatement) => {
						const callExpression = getPushCallForIdentifier(pushStatement.expression, arrayIdentifierName);
						if (callExpression === undefined) return true;

						for (const argument of callExpression.arguments) {
							if (argument.type === "SpreadElement" || !isExpressionSideEffectSafe(argument)) return true;
						}

						return false;
					});

				if (!hasUnsafeArgument) {
					context.report({
						fix(fixer) {
							return createCollapseFixes(fixer, sourceCode, declarator, pushStatements, literalText);
						},
						messageId: "collapseArrayPushInitialization",
						node: statement,
					});
					continue;
				}

				context.report({
					messageId: "collapseArrayPushInitialization",
					node: statement,
					suggest: [
						{
							fix(fixer): FixReturn {
								return createCollapseFixes(fixer, sourceCode, declarator, pushStatements, literalText);
							},
							messageId: "suggestCollapseArrayPushInitialization",
						},
					],
				});
			}
		}

		return {
			BlockStatement(node): void {
				inspectPushCollapse(node.body);
			},

			NewExpression(node): void {
				if (!isGlobalArrayConstructor(sourceCode, node)) return;

				if (node.arguments.length === 0) {
					if (!options.requireExplicitGenericOnNewArray) return;

					const hasTypeArguments =
						node.typeArguments !== undefined &&
						node.typeArguments !== null &&
						node.typeArguments.params.length > 0;
					if (hasTypeArguments || hasContextualArrayAnnotation(node, sourceCode)) return;

					context.report({
						messageId: "requireExplicitGenericOnNewArray",
						node,
					});
					return;
				}

				if (node.arguments.length > 1) {
					const [firstArgument] = node.arguments;
					if (
						firstArgument !== undefined &&
						firstArgument.type !== "SpreadElement" &&
						options.environment === "roblox-ts" &&
						!isDefinitelyNonNumericExpression(firstArgument)
					) {
						return;
					}

					if (firstArgument === undefined) return;

					const literalText = buildArrayLiteralFromArguments(node.arguments, sourceCode);
					const hasSpread = node.arguments.some((argument) => argument.type === "SpreadElement");

					if (!hasSpread) {
						context.report({
							fix(fixer) {
								return fixer.replaceText(node, literalText);
							},
							messageId: "avoidConstructorEnumeration",
							node,
						});
						return;
					}

					context.report({
						messageId: "avoidConstructorEnumeration",
						node,
						suggest: [
							{
								fix(fixer): FixReturn {
									return fixer.replaceText(node, literalText);
								},
								messageId: "suggestArrayLiteral",
							},
						],
					});
					return;
				}

				const [firstArgument] = node.arguments;
				if (firstArgument === undefined) return;

				if (firstArgument.type === "SpreadElement") {
					context.report({
						messageId: "avoidSingleArgumentConstructor",
						node,
						suggest: [
							{
								fix(fixer): FixReturn {
									return fixer.replaceText(
										node,
										`[...${sourceCode.getText(firstArgument.argument)}]`,
									);
								},
								messageId: "suggestArrayLiteral",
							},
						],
					});
					return;
				}

				if (!isDefinitelyNonNumericExpression(firstArgument)) {
					if (options.environment === "roblox-ts") return;

					const hasExplicitGeneric =
						node.typeArguments !== undefined &&
						node.typeArguments !== null &&
						node.typeArguments.params.length > 0;
					if (hasExplicitGeneric) return;

					const lengthExpressionText = sourceCode.getText(firstArgument);
					context.report({
						messageId: "avoidLengthConstructorInStandard",
						node,
						suggest: [
							{
								fix(fixer): FixReturn {
									return fixer.replaceText(node, `Array.from({ length: ${lengthExpressionText} })`);
								},
								messageId: "suggestArrayFromLength",
							},
						],
					});
					return;
				}

				const singleElementLiteral = `[${sourceCode.getText(firstArgument)}]`;
				context.report({
					fix(fixer) {
						return fixer.replaceText(node, singleElementLiteral);
					},
					messageId: "avoidSingleArgumentConstructor",
					node,
				});
			},

			Program(node): void {
				inspectPushCollapse(node.body);
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Disallow array constructor element forms and enforce roblox-ts-aware constructor patterns.",
		},
		fixable: "code",
		hasSuggestions: true,
		messages: {
			avoidConstructorEnumeration:
				"Do not use Array constructor enumeration arguments. Use an array literal instead.",
			avoidLengthConstructorInStandard:
				"Length-based Array constructor is not allowed in standard mode. Prefer Array.from({ length: n }).",
			avoidSingleArgumentConstructor:
				"Single-argument Array constructor form is not allowed here. Use an array literal instead.",
			collapseArrayPushInitialization:
				"Collapse new Array<T>() + consecutive .push(...) calls into a single array literal initializer.",
			requireExplicitGenericOnNewArray:
				"new Array() must use an explicit generic argument or a contextual Array<T>/ReadonlyArray<T> annotation.",
			suggestArrayFromLength: "Replace with Array.from({ length: value }).",
			suggestArrayLiteral: "Replace constructor form with an array literal.",
			suggestCollapseArrayPushInitialization:
				"Collapse constructor + push sequence into a single array literal initializer.",
		},
		schema: [
			{
				additionalProperties: false,
				properties: {
					environment: {
						default: "standard",
						description:
							"Array constructor environment mode: 'roblox-ts' allows new Array(length); 'standard' reports it.",
						enum: ["roblox-ts", "standard"],
						type: "string",
					},
					requireExplicitGenericOnNewArray: {
						default: true,
						description:
							"When true, zero-argument new Array() requires explicit generic type arguments or contextual array typing.",
						type: "boolean",
					},
				},
				type: "object",
			},
		],
		type: "problem",
	},
});

export default noArrayConstructorElements;
