import { PredefinedFormatToCheckFunction } from "./format";
import { selectorTypeToMessageString } from "./shared";

import type { ModifiersString, SelectorsString } from "./enums";
import type { Context, NameNode, NormalizedSelector, ValidatorFunction } from "./types";

export function createValidator(
	type: SelectorsString,
	context: Context,
	allConfigs: Array<NormalizedSelector>,
): ValidatorFunction {
	const configs = allConfigs.toSorted((first, second) => {
		if (first.selectorPriority !== second.selectorPriority) return second.selectorPriority - first.selectorPriority;
		if (first.modifierWeight !== second.modifierWeight) return second.modifierWeight - first.modifierWeight;
		return 0;
	});

	return (node, modifiers = new Set<ModifiersString>()): void => {
		let originalName = "";
		if ("name" in node && typeof node.name === "string") originalName = node.name;
		else if ("value" in node && typeof node.value === "string") originalName = node.value;

		for (const config of configs) {
			if (config.filter && config.filter.regex.test(originalName) !== config.filter.match) continue;
			// oxlint-disable-next-line typescript/strict-boolean-expressions
			if (modifiers.has("requiresQuotes") && !config.modifiers?.includes("requiresQuotes")) continue;
			// oxlint-disable-next-line typescript/strict-boolean-expressions
			if (config.modifiers?.some((modifier) => !modifiers.has(modifier))) continue;

			let name: string | undefined = originalName;

			name = validateUnderscore("leading", config, name, node, originalName, context, type);
			if (name === undefined) return;

			name = validateUnderscore("trailing", config, name, node, originalName, context, type);
			if (name === undefined) return;

			name = validateAffix("prefix", config, name, node, originalName, context, type);
			if (name === undefined) return;

			name = validateAffix("suffix", config, name, node, originalName, context, type);
			if (name === undefined) return;
			if (!validateCustom(config, name, node, originalName, context, type)) return;
			if (!validatePredefinedFormat(config, name, node, originalName, context, type, modifiers)) return;

			return;
		}
	};
}

function formatReportData(
	selectorType: SelectorsString,
	options: {
		affixes?: ReadonlyArray<string>;
		count?: "one" | "two";
		custom?: NormalizedSelector["custom"];
		formats?: ReadonlyArray<string>;
		originalName: string;
		position?: "leading" | "prefix" | "suffix" | "trailing";
		processedName?: string;
	},
): Record<string, boolean | number | string | undefined> {
	const regexMatchAlternate = options.custom?.match === false ? "not match" : undefined;
	return {
		affixes: options.affixes?.join(", "),
		count: options.count,
		formats: options.formats?.join(", "),
		name: options.originalName,
		position: options.position,
		processedName: options.processedName,
		regex: options.custom?.regex.toString(),
		regexMatch: options.custom?.match === true ? "match" : regexMatchAlternate,
		type: selectorTypeToMessageString(selectorType),
	};
}

function validateUnderscore(
	position: "leading" | "trailing",
	config: NormalizedSelector,
	name: string,
	node: NameNode,
	originalName: string,
	context: Context,
	selectorType: SelectorsString,
): string | undefined {
	const option = position === "leading" ? config.leadingUnderscore : config.trailingUnderscore;
	if (option === undefined) return name;

	const hasSingleUnderscore = position === "leading" ? name.startsWith("_") : name.endsWith("_");
	const trimmedSingleUnderscore = position === "leading" ? name.slice(1) : name.slice(0, -1);

	const hasDoubleUnderscore = position === "leading" ? name.startsWith("__") : name.endsWith("__");
	const trimmedDoubleUnderscore = position === "leading" ? name.slice(2) : name.slice(0, -2);

	switch (option) {
		case "allow": {
			if (hasSingleUnderscore) return trimmedSingleUnderscore;
			return name;
		}

		case "allowDouble": {
			if (hasDoubleUnderscore) return trimmedDoubleUnderscore;
			return name;
		}

		case "allowSingleOrDouble": {
			if (hasDoubleUnderscore) return trimmedDoubleUnderscore;
			if (hasSingleUnderscore) return trimmedSingleUnderscore;
			return name;
		}

		case "forbid": {
			if (hasSingleUnderscore) {
				context.report({
					data: formatReportData(selectorType, { count: "one", originalName, position }),
					messageId: "unexpectedUnderscore",
					node,
				});
				return undefined;
			}
			return name;
		}

		case "require": {
			if (!hasSingleUnderscore) {
				context.report({
					data: formatReportData(selectorType, { count: "one", originalName, position }),
					messageId: "missingUnderscore",
					node,
				});
				return undefined;
			}
			return trimmedSingleUnderscore;
		}

		case "requireDouble": {
			if (!hasDoubleUnderscore) {
				context.report({
					data: formatReportData(selectorType, { count: "two", originalName, position }),
					messageId: "missingUnderscore",
					node,
				});
				return undefined;
			}
			return trimmedDoubleUnderscore;
		}

		default:
			return undefined;
	}
}

function validateAffix(
	position: "prefix" | "suffix",
	config: NormalizedSelector,
	name: string,
	node: NameNode,
	originalName: string,
	context: Context,
	selectorType: SelectorsString,
): string | undefined {
	const affixes = config[position];
	if (!affixes || affixes.length === 0) return name;

	for (const affix of affixes) {
		const hasAffix = position === "prefix" ? name.startsWith(affix) : name.endsWith(affix);
		if (hasAffix) {
			return position === "prefix" ? name.slice(affix.length) : name.slice(0, -affix.length);
		}
	}

	context.report({
		data: formatReportData(selectorType, { affixes, originalName, position }),
		messageId: "missingAffix",
		node,
	});
	return undefined;
}

function validateCustom(
	config: NormalizedSelector,
	name: string,
	node: NameNode,
	originalName: string,
	context: Context,
	selectorType: SelectorsString,
): boolean {
	const { custom } = config;
	if (!custom) return true;

	const result = custom.regex.test(name);
	if ((custom.match && result) || (!custom.match && !result)) return true;

	context.report({
		data: formatReportData(selectorType, { custom, originalName }),
		messageId: "satisfyCustom",
		node,
	});
	return false;
}

function validatePredefinedFormat(
	config: NormalizedSelector,
	name: string,
	node: NameNode,
	originalName: string,
	context: Context,
	selectorType: SelectorsString,
	modifiers: Set<string>,
): boolean {
	const formats = config.format;
	if (!formats || formats.length === 0) return true;

	if (!modifiers.has("requiresQuotes")) {
		for (const format of formats) {
			const checker = PredefinedFormatToCheckFunction[format];
			if (checker?.(name) === true) return true;
		}
	}

	context.report({
		data: formatReportData(selectorType, { formats, originalName, processedName: name }),
		messageId: originalName === name ? "doesNotMatchFormat" : "doesNotMatchFormatTrimmed",
		node,
	});
	return false;
}
