import {
	hasName,
	isIdentifierName,
	isJsxIdentifier,
	isMemberExpression,
	isPropertyNode,
	isStringLiteral,
	isTsQualifiedName,
	isVariableDeclarator,
} from "@oxlint-utilities/oxc-utilities";
import {
	ANOTHER_NAME_MESSAGE,
	MESSAGE_ID_REPLACE,
	MESSAGE_ID_SUGGESTION,
} from "@oxlint-utilities/prevent-abbreviations/constants";
import { isValidIdentifier } from "@oxlint-utilities/prevent-abbreviations/identifier";
import {
	getMessage,
	getNameReplacements,
	getShorthandReplacement,
	isDiscouragedReplacementName,
	isPropertyAccessAllowed,
	isShorthandIgnored,
	isUpperFirst,
	prepareOptions,
} from "@oxlint-utilities/prevent-abbreviations/replacements";
import {
	getAvailableVariableName,
	getScopes,
	isClassVariable,
	isDefaultOrNamespaceImportName,
	isObjectPropertyKey,
	isShorthandImportLocal,
	isShorthandPropertyValue,
	renameVariable,
	shouldCheckImport,
	shouldFix,
	shouldReportIdentifierAsProperty,
} from "@oxlint-utilities/prevent-abbreviations/scope";
import { defineRule } from "oxlint-plugin-utilities";

import type { IsSafe, MessageIds, PreparedOptions, VariableLike } from "@oxlint-utilities/prevent-abbreviations/types";
import type { Definition, Diagnostic, ESTree, Fix, Fixer, Scope, Variable, Visitor } from "oxlint-plugin-utilities";

const PREVENT_ABBREVIATIONS_SCHEMA = [
	{
		additionalProperties: false,
		properties: {
			allowList: {
				additionalProperties: { type: "boolean" },
				type: "object",
			},
			allowPropertyAccess: {
				items: { type: "string" },
				type: "array",
			},
			checkDefaultAndNamespaceImports: {
				enum: [false, true, "internal"],
			},
			checkFilenames: {
				type: "boolean",
			},
			checkProperties: {
				type: "boolean",
			},
			checkShorthandImports: {
				enum: [false, true, "internal"],
			},
			checkShorthandProperties: {
				type: "boolean",
			},
			checkVariables: {
				type: "boolean",
			},
			extendDefaultAllowList: {
				type: "boolean",
			},
			extendDefaultReplacements: {
				type: "boolean",
			},
			ignore: {
				items: {
					oneOf: [{ type: "object" }, { type: "string" }],
				},
				type: "array",
			},
			ignoreShorthands: {
				items: { type: "string" },
				type: "array",
			},
			replacements: {
				additionalProperties: {
					oneOf: [
						{ enum: [false] },
						{
							additionalProperties: { type: "boolean" },
							type: "object",
						},
					],
				},
				type: "object",
			},
			shorthands: {
				additionalProperties: { type: "string" },
				type: "object",
			},
		},
		type: "object",
	},
] as const;

function createIsSafeGeneratedName(scopeToNamesGeneratedByFixer: WeakMap<Scope, Set<string>>): IsSafe {
	return function isSafeGeneratedName(name: string, scopes: ReadonlyArray<Scope>): boolean {
		return scopes.every((scope) => {
			const generatedNames = scopeToNamesGeneratedByFixer.get(scope);
			return generatedNames === undefined || !generatedNames.has(name);
		});
	};
}

function isShorthandPropertyAccess(node: ESTree.IdentifierName): boolean {
	const { parent } = node;
	return (
		(isMemberExpression(parent) && parent.property === node && !parent.computed) ||
		(isTsQualifiedName(parent) && parent.right === node)
	);
}

function reportShorthandReplacement(
	node: ESTree.IdentifierName,
	replacement: string,
	isPropertyLike: boolean,
	report: (diagnostic: Diagnostic<MessageIds>) => void,
): void {
	const samples = new Array<string>();
	samples[0] = replacement;
	report({
		...getMessage(node.name, { samples, total: 1 }, isPropertyLike ? "property" : "variable"),
		node,
	});
}

function shouldSkipVariable(
	definition: Definition,
	definitionName: ESTree.IdentifierName,
	options: PreparedOptions,
): boolean {
	if (
		(isDefaultOrNamespaceImportName(definitionName) &&
			!shouldCheckImport(options.checkDefaultAndNamespaceImports, definition)) ||
		(isShorthandImportLocal(definitionName) && !shouldCheckImport(options.checkShorthandImports, definition))
	) {
		return true;
	}

	return !options.checkShorthandProperties && isShorthandPropertyValue(definitionName);
}

function getSpecialCaseReplacement(variable: VariableLike): string | undefined {
	if (variable.name !== "plr") return undefined;

	const [definition] = variable.defs;
	if (definition?.type !== "Variable" || !isVariableDeclarator(definition.node) || definition.node.init === null) {
		return undefined;
	}

	const { init } = definition.node;
	if (
		isMemberExpression(init) &&
		!init.computed &&
		isIdentifierName(init.object) &&
		init.object.name === "Players" &&
		isIdentifierName(init.property) &&
		init.property.name === "LocalPlayer"
	) {
		return "localPlayer";
	}

	return undefined;
}

interface SafeSamplesResult {
	readonly droppedDiscouraged: number;
	readonly safeSamples: ReadonlyArray<string>;
}

function computeSafeSamples(
	samples: ReadonlyArray<string>,
	scopes: ReadonlyArray<Scope>,
	isSafeNameForVariable: IsSafe,
	options: PreparedOptions,
): SafeSamplesResult {
	const safeSamples = new Array<string>();
	let safeSamplesSize = 0;
	let droppedDiscouraged = 0;

	for (const name of samples) {
		const safeName = getAvailableVariableName(name, scopes, isSafeNameForVariable);
		if (safeName === undefined) continue;
		if (safeName !== name && isDiscouragedReplacementName(name, options)) {
			droppedDiscouraged += 1;
			continue;
		}
		if (safeName.length > 0) safeSamples[safeSamplesSize++] = safeName;
	}

	return { droppedDiscouraged, safeSamples };
}

function createIsSafeNameForVariable(
	definition: Definition,
	variable: VariableLike,
	isSafeGeneratedName: IsSafe,
): IsSafe {
	const avoidArgumentsReplacement =
		definition.type === "Variable" && isVariableDeclarator(definition.node) && definition.node.init === null;
	const avoidArgumentsInArrowParameter =
		definition.type === "Parameter" &&
		variable.scope.type === "function" &&
		variable.scope.block.type === "ArrowFunctionExpression";
	const shouldAvoidArguments = avoidArgumentsReplacement || avoidArgumentsInArrowParameter;

	return (name, scopes) => {
		if (!isSafeGeneratedName(name, scopes)) return false;
		if (shouldAvoidArguments && name === "arguments") return false;
		return true;
	};
}

function tryReportFix(
	report: (diagnostic: Diagnostic<MessageIds>) => void,
	message: { data: Record<string, string>; messageId: MessageIds },
	variable: VariableLike,
	replacement: string,
	scopes: ReadonlyArray<Scope>,
	scopeToNamesGeneratedByFixer: WeakMap<Scope, Set<string>>,
	definitionName: ESTree.IdentifierName,
): void {
	for (const scope of scopes) {
		if (!scopeToNamesGeneratedByFixer.has(scope)) {
			scopeToNamesGeneratedByFixer.set(scope, new Set());
		}
		const generatedNames = scopeToNamesGeneratedByFixer.get(scope);
		generatedNames?.add(replacement);
	}
	report({
		...message,
		fix(fixer: Fixer): Array<Fix> {
			return renameVariable(variable, replacement, fixer);
		},
		node: definitionName,
	});
}

function checkVariable(
	variable: VariableLike,
	options: PreparedOptions,
	scopeToNamesGeneratedByFixer: WeakMap<Scope, Set<string>>,
	isSafeGeneratedName: IsSafe,
	report: (diagnostic: Diagnostic<MessageIds>) => void,
): void {
	const [definition] = variable.defs;
	if (definition === undefined) return;

	const definitionName = definition.name;
	if (!isIdentifierName(definitionName)) return;
	if (shouldSkipVariable(definition, definitionName, options)) return;

	const isSafeNameForVariable = createIsSafeNameForVariable(definition, variable, isSafeGeneratedName);

	const specialCaseReplacement = getSpecialCaseReplacement(variable);
	const variableReplacements =
		specialCaseReplacement === undefined
			? getNameReplacements(variable.name, options)
			: { samples: [specialCaseReplacement], total: 1 };
	if (variableReplacements.total === 0 || !variableReplacements.samples) return;

	const { references } = variable;
	const scopes = [...references.map((reference) => reference.from), variable.scope];

	const { droppedDiscouraged, safeSamples } = computeSafeSamples(
		variableReplacements.samples,
		scopes,
		isSafeNameForVariable,
		options,
	);

	const baseSamples = safeSamples.length > 0 ? safeSamples : variableReplacements.samples;
	const hasCompleteSamples =
		typeof variableReplacements.samples.length === "number" &&
		variableReplacements.samples.length === variableReplacements.total;
	const effectiveTotal = hasCompleteSamples
		? Math.max(0, variableReplacements.total - droppedDiscouraged)
		: variableReplacements.total;
	const messageSamples =
		variable.name === "fn" && effectiveTotal > 1
			? baseSamples.map((name) => (name === "function_" ? "function" : name))
			: baseSamples;

	const message = getMessage(definitionName.name, { samples: messageSamples, total: effectiveTotal }, "variable");

	if (effectiveTotal === 1 && safeSamples.length === 1 && shouldFix(variable)) {
		const [replacement] = safeSamples;
		if (replacement !== undefined) {
			tryReportFix(report, message, variable, replacement, scopes, scopeToNamesGeneratedByFixer, definitionName);
			return;
		}
	}

	report({ ...message, node: definitionName });
}

function checkPossiblyWeirdClassVariable(variable: Variable, variableChecker: (variable: VariableLike) => void): void {
	if (!isClassVariable(variable)) {
		variableChecker(variable);
		return;
	}

	if (variable.scope.type === "class") {
		const [definition] = variable.defs;
		if (definition === undefined) {
			variableChecker(variable);
			return;
		}
		const definitionName = definition.name;
		if (!isIdentifierName(definitionName)) {
			variableChecker(variable);
			return;
		}
		variableChecker(variable);
	}
}

function checkScope(scope: Scope, variableChecker: (variable: VariableLike) => void): void {
	for (const scopeItem of getScopes(scope)) {
		for (const variable of scopeItem.variables) checkPossiblyWeirdClassVariable(variable, variableChecker);
	}
}

const preventAbbreviations = defineRule({
	create(context): Visitor {
		const options = prepareOptions(context.options[0]);
		const filenameWithExtension = context.physicalFilename;
		const scopeToNamesGeneratedByFixer = new WeakMap<Scope, Set<string>>();
		const isSafeGeneratedName = createIsSafeGeneratedName(scopeToNamesGeneratedByFixer);
		const { sourceCode } = context;

		function variableChecker(variable: VariableLike): void {
			checkVariable(variable, options, scopeToNamesGeneratedByFixer, isSafeGeneratedName, context.report);
		}

		return {
			Identifier(node): void {
				if (!hasName(node) || node.name === "__proto__") return;

				const shorthandReplacement = getShorthandReplacement(node.name, options.shorthandConfiguration);
				const propertyLike = shouldReportIdentifierAsProperty(node);
				const propertyAccess = isShorthandPropertyAccess(node);

				if (shorthandReplacement !== undefined && (propertyLike || propertyAccess)) {
					if (isShorthandIgnored(node.name, options.shorthandConfiguration)) return;
					if (!options.checkShorthandProperties) return;
					if (
						propertyAccess &&
						isPropertyAccessAllowed(node.name, shorthandReplacement, options.allowPropertyAccess)
					) {
						return;
					}

					reportShorthandReplacement(node, shorthandReplacement.replaced, true, context.report);
					return;
				}

				if (!options.checkProperties) return;

				const replacements = getNameReplacements(node.name, options);
				if (replacements.total === 0 || !propertyLike) return;

				const message = getMessage(node.name, replacements, "property");

				if (replacements.total === 1 && replacements.samples && isObjectPropertyKey(node)) {
					const [replacement] = replacements.samples;
					const { parent } = node;
					if (
						replacement !== undefined &&
						isPropertyNode(parent) &&
						isStringLiteral(parent.value) &&
						isValidIdentifier(replacement)
					) {
						context.report({
							...message,
							fix(fixer: Fixer): Fix {
								return fixer.replaceText(node, replacement);
							},
							node,
						});
						return;
					}
				}

				context.report({ ...message, node });
			},
			JSXOpeningElement({ name }): void {
				if (!options.checkVariables || !isJsxIdentifier(name) || !isUpperFirst(name.name)) return;

				const replacements = getNameReplacements(name.name, options);
				if (replacements.total === 0) return;

				const message = getMessage(name.name, replacements, "variable");
				context.report({ ...message, node: name });
			},
			"Program:exit": function (program): void {
				if (
					options.checkFilenames &&
					filenameWithExtension !== "<input>" &&
					filenameWithExtension !== "<text>"
				) {
					const lastSeparator = Math.max(
						filenameWithExtension.lastIndexOf("/"),
						filenameWithExtension.lastIndexOf("\\"),
					);
					const filename = filenameWithExtension.slice(lastSeparator + 1);
					const lastDot = filename.lastIndexOf(".");
					const extension = lastDot === -1 ? "" : filename.slice(lastDot);
					const basename = lastDot === -1 ? filename : filename.slice(0, lastDot);
					const filenameReplacements = getNameReplacements(basename, options);
					if (filenameReplacements.total > 0 && filenameReplacements.samples) {
						const samples = filenameReplacements.samples.map((replacement) => `${replacement}${extension}`);
						context.report({
							...getMessage(filename, { samples, total: filenameReplacements.total }, "filename"),
							node: program,
						});
					}
				}

				if (!options.checkVariables) return;
				checkScope(sourceCode.getScope(program), variableChecker);
			},
		} satisfies Visitor;
	},
	meta: {
		docs: {
			description: "Prevent abbreviations.",
			recommended: false,
		},
		fixable: "code",
		messages: {
			[MESSAGE_ID_REPLACE]: `The {{nameTypeText}} \`{{discouragedName}}\` should be named \`{{replacement}}\`. ${ANOTHER_NAME_MESSAGE}`,
			[MESSAGE_ID_SUGGESTION]: `Please rename the {{nameTypeText}} \`{{discouragedName}}\`. Suggested names are: {{replacementsText}}. ${ANOTHER_NAME_MESSAGE}`,
		},
		schema: PREVENT_ABBREVIATIONS_SCHEMA,
		type: "suggestion",
	},
});

export default preventAbbreviations;
