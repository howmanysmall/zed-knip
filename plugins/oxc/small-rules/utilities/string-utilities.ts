const UPPERCASE_PATTERN = /^[A-Z]/v;

export function isUppercaseName(name: string): boolean {
	return UPPERCASE_PATTERN.test(name);
}
