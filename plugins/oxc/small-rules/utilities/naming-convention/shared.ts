import { MetaSelectors } from "./enums";

import type { IndividualAndMetaSelectorsString, MetaSelectorsString } from "./enums";

const ALPHABETICAL = /([A-Z])/gu;

export function selectorTypeToMessageString(selectorType: string): string {
	const notCamelCase = selectorType.replaceAll(ALPHABETICAL, " $1");
	return notCamelCase.charAt(0).toUpperCase() + notCamelCase.slice(1);
}

export function isMetaSelector(selector: IndividualAndMetaSelectorsString): selector is MetaSelectorsString {
	return selector in MetaSelectors;
}

export function isMethodOrPropertySelector(selector: IndividualAndMetaSelectorsString): boolean {
	return selector === "method" || selector === "property";
}
