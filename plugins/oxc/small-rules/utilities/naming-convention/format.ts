function isUppercaseCharacter(character: string): boolean {
	return character === character.toUpperCase() && character !== character.toLowerCase();
}

function isPascalCase(name: string): boolean {
	if (name.length === 0) return true;
	const [first] = name;
	return first === undefined ? false : first === first.toUpperCase() && !name.includes("_");
}

function isStrictPascalCase(name: string): boolean {
	if (name.length === 0) return true;
	const [first] = name;
	return first === undefined ? false : first === first.toUpperCase() && hasStrictCamelHumps(name, true);
}

function isCamelCase(name: string): boolean {
	if (name.length === 0) return true;

	const [first] = name;
	return first === undefined ? false : first === first.toLowerCase() && !name.includes("_");
}

function isStrictCamelCase(name: string): boolean {
	if (name.length === 0) return true;

	const [first] = name;
	return first === undefined ? false : first === first.toLowerCase() && hasStrictCamelHumps(name, false);
}

function hasStrictCamelHumps(name: string, isUpper: boolean): boolean {
	if (name.startsWith("_")) return false;

	let isUpperCase = isUpper;
	for (let index = 1; index < name.length; index += 1) {
		const character = name[index];
		if (character === undefined || character === "_") return false;

		if (isUpperCase === isUppercaseCharacter(character)) {
			if (isUpperCase) return false;
		} else isUpperCase = !isUpperCase;
	}
	return true;
}

function isSnakeCase(name: string): boolean {
	return name.length === 0 || (name === name.toLowerCase() && validateUnderscores(name));
}

function isStringUpperCase(name: string): boolean {
	return name.length === 0 || (name === name.toUpperCase() && validateUnderscores(name));
}

function validateUnderscores(name: string): boolean {
	if (name.startsWith("_")) return false;

	let wasUnderscore = false;
	for (let index = 1; index < name.length; index += 1) {
		const character = name[index];
		if (character === undefined) return false;
		if (character === "_") {
			if (wasUnderscore) return false;
			wasUnderscore = true;
		} else wasUnderscore = false;
	}
	return !wasUnderscore;
}

export const PredefinedFormatToCheckFunction: Record<string, (name: string) => boolean> = {
	camelCase: isCamelCase,
	PascalCase: isPascalCase,
	snake_case: isSnakeCase,
	strictCamelCase: isStrictCamelCase,
	StrictPascalCase: isStrictPascalCase,
	UPPER_CASE: isStringUpperCase,
};
