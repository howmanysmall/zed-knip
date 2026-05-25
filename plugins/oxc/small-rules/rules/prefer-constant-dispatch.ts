import { getVariableByName, unwrapExpression } from "@oxlint-utilities/ast-utilities";
import { getHookName } from "@oxlint-utilities/react-hook-utilities";
import { isImportBinding, isModuleLevelScope } from "@oxlint-utilities/static-expression-utilities";
import { defineRule } from "oxlint-plugin-utilities";

import type { ScopeVariable } from "@oxlint-utilities/ast-utilities";
import type { ESTree, Fix, Fixer, SourceCode, Visitor } from "oxlint-plugin-utilities";

function isModuleScopeConst(variable: ScopeVariable): boolean {
	if (!isModuleLevelScope(variable.scope)) return false;

	for (const definition of variable.defs) {
		if (definition.type !== "Variable") continue;

		const declarator = definition.node;
		if (declarator.type !== "VariableDeclarator") continue;

		const declaration = declarator.parent;
		if (declaration.type === "VariableDeclaration" && declaration.kind === "const") return true;
	}

	return false;
}

function resolvesToConstantIdentifier(sourceCode: SourceCode, identifier: ESTree.IdentifierReference): boolean {
	const variable = getVariableByName(sourceCode.getScope(identifier), identifier.name);
	if (variable === undefined) return false;

	return isImportBinding(variable) || isModuleScopeConst(variable);
}

function resolvesToModuleScopeBinding(sourceCode: SourceCode, identifier: ESTree.IdentifierReference): boolean {
	const variable = getVariableByName(sourceCode.getScope(identifier), identifier.name);
	if (variable === undefined) return false;

	return isImportBinding(variable) || isModuleLevelScope(variable.scope);
}

function isConstantMemberExpression(sourceCode: SourceCode, memberExpression: ESTree.MemberExpression): boolean {
	let current: ESTree.Expression = memberExpression;

	while (current.type === "MemberExpression") {
		if (current.computed) return false;
		current = unwrapExpression(current.object);
	}

	return current.type === "Identifier" && resolvesToModuleScopeBinding(sourceCode, current);
}

function isConstantDispatchValue(sourceCode: SourceCode, expression: ESTree.Expression): boolean {
	const unwrapped = unwrapExpression(expression);

	switch (unwrapped.type) {
		case "Identifier":
			return resolvesToConstantIdentifier(sourceCode, unwrapped);

		case "Literal":
			return true;

		case "MemberExpression":
			return isConstantMemberExpression(sourceCode, unwrapped);

		case "TemplateLiteral":
			return unwrapped.expressions.length === 0;

		case "UnaryExpression":
			return unwrapped.operator === "-" && unwrapExpression(unwrapped.argument).type === "Literal";

		default:
			return false;
	}
}

function shouldReportActionObject(sourceCode: SourceCode, objectExpression: ESTree.ObjectExpression): boolean {
	for (const property of objectExpression.properties) {
		if (property.type !== "Property") return false;
		if (property.kind !== "init" || property.computed) return false;
		if (!isConstantDispatchValue(sourceCode, property.value)) return false;
	}

	return true;
}

function getTrackedDispatchVariable(
	sourceCode: SourceCode,
	variableDeclarator: ESTree.VariableDeclarator,
): ScopeVariable | undefined {
	if (variableDeclarator.id.type !== "ArrayPattern" || variableDeclarator.init?.type !== "CallExpression") {
		return undefined;
	}

	if (getHookName(variableDeclarator.init) !== "useReducer") return undefined;

	// oxlint-disable-next-line prefer-destructuring
	const dispatchElement = variableDeclarator.id.elements[1];
	if (dispatchElement === undefined || dispatchElement === null) return undefined;
	if (dispatchElement.type !== "Identifier") return undefined;

	return getVariableByName(sourceCode.getScope(dispatchElement), dispatchElement.name);
}

function getProgram(node: ESTree.Node): ESTree.Program | undefined {
	let current: ESTree.Node | undefined = node;

	// oxlint-disable-next-line typescript/no-unnecessary-condition
	while (current !== undefined) {
		if (current.type === "Program") return current;
		current = current.parent;
	}

	return undefined;
}

function getDeclarationInsertionFix(
	fixer: Fixer,
	sourceCode: SourceCode,
	actionObject: ESTree.ObjectExpression,
	constantName: string,
	program: ESTree.Program,
): Fix {
	const declarationText = `const ${constantName} = ${sourceCode.getText(actionObject)};`;
	const { body } = program;

	let lastImport: ESTree.ImportDeclaration | undefined;
	for (const statement of body) {
		if (statement.type === "ImportDeclaration") {
			lastImport = statement;
			continue;
		}
		break;
	}

	if (lastImport !== undefined) return fixer.insertTextAfter(lastImport, `\n\n${declarationText}`);

	const [firstStatement] = body;
	if (firstStatement !== undefined) {
		return fixer.insertTextBefore(firstStatement, `${declarationText}\n\n`);
	}

	return fixer.insertTextAfterRange([program.range[0], program.range[1]], declarationText);
}

function createSuggestionFixes(
	fixer: Fixer,
	sourceCode: SourceCode,
	actionObject: ESTree.ObjectExpression,
	constantName: string,
	program: ESTree.Program,
): Array<Fix> {
	return [
		getDeclarationInsertionFix(fixer, sourceCode, actionObject, constantName, program),
		fixer.replaceText(actionObject, constantName),
	];
}

const preferConstantDispatch = defineRule({
	create(context): Visitor {
		const trackedDispatchVariables = new Set<ScopeVariable>();
		const { sourceCode } = context;
		let suggestionCount = 0;

		return {
			CallExpression(node): void {
				if (node.callee.type !== "Identifier") return;

				const dispatchVariable = getVariableByName(sourceCode.getScope(node.callee), node.callee.name);
				if (dispatchVariable === undefined || !trackedDispatchVariables.has(dispatchVariable)) return;

				const [firstArgument] = node.arguments;
				if (firstArgument === undefined || firstArgument.type === "SpreadElement") return;

				const actionArgument = unwrapExpression(firstArgument);
				if (
					actionArgument.type !== "ObjectExpression" ||
					!shouldReportActionObject(sourceCode, actionArgument)
				) {
					return;
				}

				const program = getProgram(actionArgument);
				if (program === undefined) return;

				const constantName = `PREFER_CONSTANT_ACTION_${suggestionCount}`;
				suggestionCount += 1;

				context.report({
					messageId: "preferConstantDispatch",
					node: actionArgument,
					suggest: [
						{
							desc: `Extract to module constant \`${constantName}\``,
							fix(fixer): Array<Fix> {
								return createSuggestionFixes(fixer, sourceCode, actionArgument, constantName, program);
							},
						},
					],
				});
			},
			VariableDeclarator(node): void {
				const dispatchVariable = getTrackedDispatchVariable(sourceCode, node);
				if (dispatchVariable !== undefined) trackedDispatchVariables.add(dispatchVariable);
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Disallow inline useReducer action objects that could be module-level constants.",
			recommended: true,
		},
		hasSuggestions: true,
		messages: {
			preferConstantDispatch: "Move this inline useReducer action object to a module-level constant.",
		},
		schema: [] as const,
		type: "suggestion",
	},
});

export default preferConstantDispatch;
