import { type } from "arktype";

import {
	isIndividualAndMetaSelectorsString,
	isModifiersString,
	isPredefinedFormatsString,
	isUnderscoreOptionsString,
} from "./enums";

import type { ESTree, Context as RuleContext } from "oxlint-plugin-utilities";

import type { ModifiersString, PredefinedFormatsString, UnderscoreOptionsString } from "./enums";

export type MessageIds =
	| "doesNotMatchFormat"
	| "doesNotMatchFormatTrimmed"
	| "missingAffix"
	| "missingUnderscore"
	| "satisfyCustom"
	| "unexpectedUnderscore";

export const isMatchRegex = type({
	match: "boolean",
	regex: "string",
}).readonly();
export type MatchRegex = typeof isMatchRegex.infer;

export const isSelectorArray = isIndividualAndMetaSelectorsString.array().readonly();
export const isSelector = type({
	"custom?": isMatchRegex.or("undefined"),
	"filter?": isMatchRegex.or("string | undefined"),
	"format?": type("null | undefined").or(isPredefinedFormatsString.array().readonly()),
	"leadingUnderscore?": isUnderscoreOptionsString.or("undefined"),
	"modifiers?": isModifiersString.array().readonly().or("undefined"),
	"prefix?": type("string[]").readonly().or("undefined"),
	selector: isIndividualAndMetaSelectorsString.or(isSelectorArray),
	"suffix?": type("string[]").readonly().or("undefined"),
	"trailingUnderscore?": isUnderscoreOptionsString.or("undefined"),
}).readonly();
export type Selector = typeof isSelector.infer;

export interface NormalizedMatchRegex {
	match: boolean;
	regex: RegExp;
}

export interface NormalizedSelector {
	custom?: NormalizedMatchRegex | undefined;
	filter?: NormalizedMatchRegex | undefined;
	format?: null | ReadonlyArray<PredefinedFormatsString> | undefined;
	leadingUnderscore?: undefined | UnderscoreOptionsString;
	modifiers?: ReadonlyArray<ModifiersString> | undefined;
	modifierWeight: number;
	prefix?: ReadonlyArray<string> | undefined;
	selectorPriority: number;
	selectors: ReadonlyArray<string>;
	suffix?: ReadonlyArray<string> | undefined;
	trailingUnderscore?: undefined | UnderscoreOptionsString;
}

export type NameNode =
	| ESTree.BindingIdentifier
	| ESTree.IdentifierName
	| ESTree.PrivateIdentifier
	| ESTree.StringLiteral
	| ESTree.TemplateLiteral;

export type ValidatorFunction = (node: NameNode, modifiers?: Set<string>) => void;

export type ParsedOptions = Record<string, ValidatorFunction>;

export const isOptions = isSelector.array().readonly();
export type Options = typeof isOptions.infer;

export type Context = Readonly<RuleContext<ReadonlyArray<Selector>, MessageIds>>;
