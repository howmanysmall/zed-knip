import { MetaSelectors, ModifierWeights, Selectors } from "./enums";
import { getEnumNames } from "./get-enum-names";
import { isMetaSelector, isMethodOrPropertySelector } from "./shared";
import { isSelectorArray } from "./types";
import { createValidator } from "./validator";

import type { Except } from "type-fest";

import type { IndividualAndMetaSelectorsString, MetaSelectorsString } from "./enums";
import type {
	Context,
	NameNode,
	NormalizedMatchRegex,
	NormalizedSelector,
	Options,
	ParsedOptions,
	Selector,
} from "./types";

const META_SELECTOR_MAP: Record<MetaSelectorsString, Array<keyof typeof Selectors>> = {
	accessor: ["classicAccessor", "autoAccessor"],
	default: getEnumNames(Selectors),
	memberLike: [
		"classProperty",
		"objectLiteralProperty",
		"typeProperty",
		"parameterProperty",
		"enumMember",
		"classMethod",
		"objectLiteralMethod",
		"typeMethod",
		"classicAccessor",
		"autoAccessor",
	],
	method: ["classMethod", "objectLiteralMethod", "typeMethod"],
	property: ["classProperty", "objectLiteralProperty", "typeProperty"],
	typeLike: ["class", "interface", "typeAlias", "enum", "typeParameter"],
	variableLike: ["variable", "function", "parameter"],
};

function normalizeFilter(option: Selector): NormalizedMatchRegex | undefined {
	if (option.filter === undefined) return undefined;
	if (typeof option.filter === "string") return { match: true, regex: new RegExp(option.filter, "u") };
	return { match: option.filter.match, regex: new RegExp(option.filter.regex, "u") };
}

function normalizeCustom(option: Selector): NormalizedMatchRegex | undefined {
	if (option.custom === undefined) return undefined;
	return { match: option.custom.match, regex: new RegExp(option.custom.regex, "u") };
}

function normalizeOption(option: Selector): Array<NormalizedSelector> {
	let weight = 0;
	if (option.modifiers) for (const modifier of option.modifiers) weight += ModifierWeights[modifier];
	if (option.filter !== undefined) weight += 1_000_000_000;

	const normalizedOption: Except<NormalizedSelector, "selectorPriority" | "selectors"> = {
		modifierWeight: weight,
	};

	const custom = normalizeCustom(option);
	if (custom !== undefined) normalizedOption.custom = custom;

	const filter = normalizeFilter(option);
	if (filter !== undefined) normalizedOption.filter = filter;
	if (Array.isArray(option.format)) normalizedOption.format = option.format;

	if (option.leadingUnderscore !== undefined) normalizedOption.leadingUnderscore = option.leadingUnderscore;
	if (option.modifiers && option.modifiers.length > 0) normalizedOption.modifiers = option.modifiers;
	if (option.prefix && option.prefix.length > 0) normalizedOption.prefix = option.prefix;
	if (option.suffix && option.suffix.length > 0) normalizedOption.suffix = option.suffix;
	if (option.trailingUnderscore !== undefined) normalizedOption.trailingUnderscore = option.trailingUnderscore;

	const selectors = isSelectorArray.allows(option.selector) ? option.selector : [option.selector];
	const normalizedSelectors = new Array<NormalizedSelector>();

	for (const selector of selectors) {
		const selectorPriority = getSelectorPriority(selector);
		const resolvedSelectors = isMetaSelector(selector) ? META_SELECTOR_MAP[selector] : [Selectors[selector]];

		normalizedSelectors.push({
			...normalizedOption,
			selectorPriority,
			selectors: resolvedSelectors,
		});
	}

	return normalizedSelectors;
}

function getSelectorPriority(selector: IndividualAndMetaSelectorsString): number {
	if (selector === MetaSelectors.default) return 0;
	if (isMetaSelector(selector)) return isMethodOrPropertySelector(selector) ? 2 : 1;
	return 3;
}

export function parseOptions(options: Options, context: Context): ParsedOptions {
	const normalizedOptions = options.flatMap(normalizeOption);
	const selectorNames = getEnumNames(Selectors);

	const selectorMap = new Map<string, Array<NormalizedSelector>>();
	for (const selectorName of selectorNames) selectorMap.set(selectorName, []);

	for (const option of normalizedOptions) {
		for (const selector of option.selectors) selectorMap.get(selector)?.push(option);
	}

	const entries = selectorNames.map((selectorName): [string, (node: NameNode, modifiers?: Set<string>) => void] => [
		selectorName,
		createValidator(selectorName, context, selectorMap.get(selectorName) ?? []),
	]);
	return Object.fromEntries(entries) as ParsedOptions;
}
