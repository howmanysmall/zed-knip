import { isRecord, isStringArray, isStringRecord } from "@oxlint-utilities/type-utilities";
import { regex } from "arktype";

import {
	DEFAULT_ALLOW_LIST,
	DEFAULT_ALLOW_PROPERTY_ACCESS,
	DEFAULT_IGNORE,
	DEFAULT_REPLACEMENTS,
	DEFAULT_SHORTHANDS,
	IS_ALPHABETIC,
	MESSAGE_ID_REPLACE,
	MESSAGE_ID_SUGGESTION,
	WORD_SPLIT_PATTERN,
} from "./constants";

import type {
	ImportCheckOption,
	MessageIds,
	NameReplacements,
	PreparedOptions,
	ShorthandConfiguration,
	ShorthandMatcher,
	ShorthandReplacement,
} from "./types";

const WORD_BOUNDARY_REGEX = /(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])|(?<=[a-zA-Z])(?=\d)|(?<=\d)(?=[a-zA-Z])/u;
const SPECIAL_CHARACTER_REGEX = /[.+^${}()|[\]\\]/gu;
const REGEX_PATTERN_MATCHER = regex("^/(?<first>.+)/(?<second>[gimsuy]*)$", "v");

function isUpperCase(value: string): boolean {
	return value === value.toUpperCase();
}

export function isUpperFirst(value: string): boolean {
	return isUpperCase(value.charAt(0));
}

function upperFirst(value: string): string {
	return value.charAt(0).toUpperCase() + value.slice(1);
}

function lowerFirst(value: string): string {
	return value.charAt(0).toLowerCase() + value.slice(1);
}

function splitIdentifierIntoWords(identifier: string): ReadonlyArray<string> {
	return identifier.split(WORD_BOUNDARY_REGEX);
}

function countCaptureGroups(replacement: string): number {
	const dollarReferences = replacement.match(/\$(\d+)/gu);
	if (dollarReferences === null) return 0;

	let maxGroup = 0;
	for (const dollarReference of dollarReferences) {
		const groupNumber = Number.parseInt(dollarReference.slice(1), 10);
		if (groupNumber > maxGroup) maxGroup = groupNumber;
	}

	return maxGroup;
}

function buildReplacementPatterns(replacement: string): ReadonlyArray<RegExp> {
	const count = countCaptureGroups(replacement);
	if (count === 0) return [];

	const patterns = new Array<RegExp>(count);
	for (let index = 1; index <= count; index += 1) {
		patterns[index - 1] = new RegExp(`\\$${index}`, "gu");
	}
	return patterns;
}

type MatcherResult =
	| { matcher: ShorthandMatcher; type: "pattern" }
	| { original: string; replacement: string; type: "exact" };

function createMatcher(key: string, replacement: string): MatcherResult {
	if (key.startsWith("/")) {
		const match = REGEX_PATTERN_MATCHER.exec(key);
		if (match?.groups !== undefined) {
			return {
				matcher: {
					original: key,
					pattern: new RegExp(`^${match.groups.first}$`, match.groups.second),
					replacement,
					replacementPatterns: buildReplacementPatterns(replacement),
				},
				type: "pattern",
			};
		}
	}

	if (key.includes("*") || key.includes("?")) {
		const regexPattern = key
			.replaceAll(SPECIAL_CHARACTER_REGEX, String.raw`\$&`)
			.replaceAll("*", "(.*)")
			.replaceAll("?", "(.)");

		let captureIndex = 0;
		const regexReplacement = replacement.replaceAll("*", () => `$${++captureIndex}`);

		return {
			matcher: {
				original: key,
				pattern: new RegExp(`^${regexPattern}$`, "u"),
				replacement: regexReplacement,
				replacementPatterns: buildReplacementPatterns(regexReplacement),
			},
			type: "pattern",
		};
	}

	return { original: key, replacement, type: "exact" };
}

function matchWord(
	word: string,
	configuration: ShorthandConfiguration,
): ShorthandReplacement["matches"][number] | undefined {
	const exactReplacement = configuration.exactMatchers.get(word);
	if (exactReplacement !== undefined) {
		return {
			matchedWord: word,
			replacement: exactReplacement,
			shorthand: word,
		};
	}

	for (const matcher of configuration.matchers) {
		const match = word.match(matcher.pattern);
		if (match === null) continue;

		let replaced = matcher.replacement;
		let captureIndex = 1;
		for (const replacementPattern of matcher.replacementPatterns) {
			replaced = replaced.replaceAll(replacementPattern, match[captureIndex] ?? "");
			captureIndex += 1;
		}

		return {
			matchedWord: word,
			replacement: replaced,
			shorthand: matcher.original,
		};
	}

	return undefined;
}

function isWordIgnored(word: string, configuration: ShorthandConfiguration): boolean {
	if (configuration.ignoreExact.has(word)) return true;

	for (const matcher of configuration.ignoreMatchers) {
		if (matcher.pattern.test(word)) return true;
	}

	return false;
}

function normalizeImportCheckOption(value: unknown, defaultValue: ImportCheckOption): ImportCheckOption {
	if (value === "internal" || typeof value === "boolean") return value;
	return defaultValue;
}

function normalizeBooleanRecord(value: unknown): Record<string, boolean> | undefined {
	if (!isRecord(value)) return undefined;

	const normalizedValue: Record<string, boolean> = {};
	for (const [key, entry] of Object.entries(value)) {
		if (typeof entry === "boolean") normalizedValue[key] = entry;
	}

	return normalizedValue;
}

function normalizeBooleanOption(value: unknown, defaultValue: boolean): boolean {
	return typeof value === "boolean" ? value : defaultValue;
}

function normalizeAllowList(options: unknown): Map<string, boolean> {
	const normalizedOptions = isRecord(options) ? options : undefined;
	const configuredAllowList = normalizeBooleanRecord(normalizedOptions?.allowList);
	const extendDefaultAllowList = normalizeBooleanOption(normalizedOptions?.extendDefaultAllowList, true);
	const mergedAllowList = extendDefaultAllowList
		? { ...DEFAULT_ALLOW_LIST, ...configuredAllowList }
		: (configuredAllowList ?? {});

	return new Map(Object.entries(mergedAllowList));
}

function normalizeReplacementOverrides(value: unknown): Record<string, false | Record<string, boolean>> {
	if (!isRecord(value)) return {};

	const overrides: Record<string, false | Record<string, boolean>> = {};
	for (const [name, override] of Object.entries(value)) {
		if (override === false) {
			overrides[name] = false;
			continue;
		}

		if (!isRecord(override)) continue;

		const normalizedOverride: Record<string, boolean> = {};
		for (const [replacementName, enabled] of Object.entries(override)) {
			if (typeof enabled === "boolean") normalizedOverride[replacementName] = enabled;
		}
		overrides[name] = normalizedOverride;
	}

	return overrides;
}

function normalizeReplacements(options: unknown): Map<string, Map<string, boolean>> {
	const normalizedOptions = isRecord(options) ? options : undefined;
	const extendDefaultReplacements = normalizeBooleanOption(normalizedOptions?.extendDefaultReplacements, true);
	const configuredReplacements = normalizeReplacementOverrides(normalizedOptions?.replacements);
	const replacementKeys = new Set<string>([
		...Object.keys(DEFAULT_REPLACEMENTS),
		...Object.keys(configuredReplacements),
	]);

	const mergedReplacements = new Array<[string, Map<string, boolean>]>();

	for (const discouragedName of replacementKeys) {
		const configuredOverride = configuredReplacements[discouragedName];
		const baseReplacements = extendDefaultReplacements ? (DEFAULT_REPLACEMENTS[discouragedName] ?? {}) : {};
		const mergedForName = configuredOverride === false ? {} : { ...baseReplacements, ...configuredOverride };
		mergedReplacements.push([discouragedName, new Map(Object.entries(mergedForName))]);
	}

	return new Map(mergedReplacements);
}

function normalizeIgnorePatterns(options: unknown): ReadonlyArray<RegExp> {
	const normalizedOptions = isRecord(options) ? options : undefined;
	const ignorePatterns = new Array<RegExp | string>();

	for (const pattern of DEFAULT_IGNORE) ignorePatterns.push(pattern);

	if (Array.isArray(normalizedOptions?.ignore)) {
		for (const pattern of normalizedOptions.ignore) {
			if (typeof pattern === "string" || pattern instanceof RegExp) ignorePatterns.push(pattern);
		}
	}

	return ignorePatterns.map((pattern) => (pattern instanceof RegExp ? pattern : new RegExp(pattern, "u")));
}

function normalizeAllowPropertyAccess(options: unknown): ReadonlySet<string> {
	const normalizedOptions = isRecord(options) ? options : undefined;
	const allowPropertyAccess = new Set<string>(DEFAULT_ALLOW_PROPERTY_ACCESS);
	if (!isStringArray(normalizedOptions?.allowPropertyAccess)) return allowPropertyAccess;

	for (const name of normalizedOptions.allowPropertyAccess) allowPropertyAccess.add(name);
	return allowPropertyAccess;
}

function normalizeShorthandConfiguration(options: unknown): ShorthandConfiguration {
	const normalizedOptions = isRecord(options) ? options : undefined;
	const exactMatchers = new Map<string, string>();
	const ignoreExact = new Set<string>();
	const ignoreMatchers = new Array<ShorthandMatcher>();
	const matchers = new Array<ShorthandMatcher>();
	const configuredShorthands = isStringRecord(normalizedOptions?.shorthands) ? normalizedOptions.shorthands : {};
	const mergedShorthands = { ...DEFAULT_SHORTHANDS, ...configuredShorthands };

	for (const [key, value] of Object.entries(mergedShorthands)) {
		const result = createMatcher(key, value);
		if (result.type === "exact") exactMatchers.set(result.original, result.replacement);
		else matchers.push(result.matcher);
	}

	if (isStringArray(normalizedOptions?.ignoreShorthands)) {
		for (const pattern of normalizedOptions.ignoreShorthands) {
			const result = createMatcher(pattern, "");
			if (result.type === "exact") ignoreExact.add(result.original);
			else ignoreMatchers.push(result.matcher);
		}
	}

	return {
		exactMatchers,
		ignoreExact,
		ignoreMatchers,
		matchers,
	};
}

export function prepareOptions(options: unknown): PreparedOptions {
	const normalizedOptions = isRecord(options) ? options : undefined;
	return {
		allowList: normalizeAllowList(normalizedOptions),
		allowPropertyAccess: normalizeAllowPropertyAccess(normalizedOptions),
		checkDefaultAndNamespaceImports: normalizeImportCheckOption(
			normalizedOptions?.checkDefaultAndNamespaceImports,
			"internal",
		),
		checkFilenames: normalizeBooleanOption(normalizedOptions?.checkFilenames, true),
		checkProperties: normalizeBooleanOption(normalizedOptions?.checkProperties, false),
		checkShorthandImports: normalizeImportCheckOption(normalizedOptions?.checkShorthandImports, "internal"),
		checkShorthandProperties: normalizeBooleanOption(normalizedOptions?.checkShorthandProperties, true),
		checkVariables: normalizeBooleanOption(normalizedOptions?.checkVariables, true),
		ignore: normalizeIgnorePatterns(normalizedOptions),
		replacements: normalizeReplacements(normalizedOptions),
		shorthandConfiguration: normalizeShorthandConfiguration(normalizedOptions),
	};
}

function getWordReplacements(word: string, options: PreparedOptions): ReadonlyArray<string> {
	if (isUpperCase(word) || options.allowList.get(word) === true) return [];

	const replacement =
		options.replacements.get(lowerFirst(word)) ??
		options.replacements.get(word) ??
		options.replacements.get(upperFirst(word));

	if (replacement === undefined) return [];

	const transform = isUpperFirst(word) ? upperFirst : lowerFirst;
	const wordReplacement = [...replacement.keys()].filter((name) => replacement.get(name) === true).map(transform);

	return wordReplacement.length > 0 ? wordReplacement.toSorted() : [];
}

export function getShorthandReplacement(
	identifier: string,
	configuration: ShorthandConfiguration,
): ShorthandReplacement | undefined {
	if (identifier.length === 0) return undefined;

	const words = splitIdentifierIntoWords(identifier);
	const matches = new Array<ShorthandReplacement["matches"][number]>();
	let hasMatch = false;

	for (const word of words) {
		const match = matchWord(word, configuration);
		if (match === undefined) continue;
		hasMatch = true;
		matches.push(match);
	}

	if (!hasMatch) return undefined;

	let replaced = "";
	let matchIndex = 0;
	for (const word of words) {
		const currentMatch = matches[matchIndex];
		if (currentMatch?.matchedWord === word) {
			replaced += currentMatch.replacement;
			matchIndex += 1;
			continue;
		}

		replaced += word;
	}

	return { matches, replaced };
}

export function isShorthandIgnored(identifier: string, configuration: ShorthandConfiguration): boolean {
	if (isWordIgnored(identifier, configuration)) return true;

	const words = splitIdentifierIntoWords(identifier);
	let hasRelevantMatch = false;
	for (const word of words) {
		const match = matchWord(word, configuration);
		if (match === undefined) continue;
		hasRelevantMatch = true;
		if (!isWordIgnored(match.matchedWord, configuration)) return false;
	}

	return hasRelevantMatch;
}

export function isPropertyAccessAllowed(
	identifier: string,
	replacement: ShorthandReplacement,
	allowPropertyAccess: ReadonlySet<string>,
): boolean {
	if (allowPropertyAccess.has(identifier)) return true;

	for (const match of replacement.matches) {
		if (!allowPropertyAccess.has(match.matchedWord)) return false;
	}

	return replacement.matches.length > 0;
}

export function getNameReplacements(name: string, options: PreparedOptions, limit = 3): NameReplacements {
	if (
		isUpperCase(name) ||
		options.allowList.get(name) === true ||
		options.ignore.some((pattern) => pattern.test(name))
	) {
		return { total: 0 };
	}

	const shorthandReplacement = getShorthandReplacement(name, options.shorthandConfiguration);
	if (shorthandReplacement !== undefined) {
		if (isShorthandIgnored(name, options.shorthandConfiguration)) return { total: 0 };
		return {
			samples: [shorthandReplacement.replaced],
			total: 1,
		};
	}

	const exactReplacements = getWordReplacements(name, options);
	if (exactReplacements.length > 0) {
		return {
			samples: exactReplacements.slice(0, limit),
			total: exactReplacements.length,
		};
	}

	const words = name.split(WORD_SPLIT_PATTERN).filter(Boolean);
	let hasReplacements = false;
	const combinations = new Array<ReadonlyArray<string>>();
	let size = 0;

	for (const word of words) {
		const wordReplacements = getWordReplacements(word, options);
		if (wordReplacements.length > 0) {
			hasReplacements = true;
			combinations[size++] = wordReplacements;
			continue;
		}

		combinations[size++] = [word];
	}

	if (!hasReplacements) return { total: 0 };

	const total = combinations.reduce((count, entries) => count * entries.length, 1);
	const sampleCount = Math.min(total, limit);
	const samples = Array.from({ length: sampleCount }, (_, sampleIndex) => {
		let indexRemaining = sampleIndex;
		const combination = new Array<string>();
		for (let combinationIndex = combinations.length - 1; combinationIndex >= 0; combinationIndex -= 1) {
			const entries = combinations[combinationIndex] ?? [];
			const index = indexRemaining % entries.length;
			indexRemaining = (indexRemaining - index) / entries.length;
			const entry = entries[index];
			if (entry !== undefined) combination.unshift(entry);
		}
		return combination;
	});

	for (const parts of samples) {
		for (let index = parts.length - 1; index > 0; index -= 1) {
			const word = parts[index] ?? "";
			if (IS_ALPHABETIC.test(word) && parts[index - 1]?.endsWith(word) === true) parts.splice(index, 1);
		}
	}

	return {
		samples: samples.map((parts) => parts.join("")),
		total,
	};
}

export function isDiscouragedReplacementName(name: string, options: PreparedOptions): boolean {
	const replacement = options.replacements.get(name);
	if (replacement === undefined) return false;

	for (const enabled of replacement.values()) {
		if (enabled) return true;
	}

	return false;
}

export function getMessage(
	discouragedName: string,
	replacements: NameReplacements,
	nameTypeText: string,
): { data: Record<string, string>; messageId: MessageIds } {
	const { samples = [], total } = replacements;

	if (total === 1) {
		return {
			data: {
				discouragedName,
				nameTypeText,
				replacement: samples[0] ?? "",
			},
			messageId: MESSAGE_ID_REPLACE,
		};
	}

	let replacementsText = samples.map((replacement) => `\`${replacement}\``).join(", ");
	const omittedReplacementsCount = total - samples.length;
	if (omittedReplacementsCount > 0) {
		replacementsText += `, ... (${omittedReplacementsCount > 99 ? "99+" : omittedReplacementsCount} more omitted)`;
	}

	return {
		data: {
			discouragedName,
			nameTypeText,
			replacementsText,
		},
		messageId: MESSAGE_ID_SUGGESTION,
	};
}
