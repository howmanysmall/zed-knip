import { TYPESCRIPT_RESERVED_WORDS } from "./constants";

function isAsciiIdentifierStart(codePoint: number): boolean {
	return (
		(codePoint >= 65 && codePoint <= 90) ||
		(codePoint >= 97 && codePoint <= 122) ||
		codePoint === 36 ||
		codePoint === 95
	);
}

export function isIdentifierStartCodePoint(codePoint: number): boolean {
	if (codePoint < 0xc0) return isAsciiIdentifierStart(codePoint);
	if (codePoint >= 0x3001 && codePoint <= 0xd7ff) return true;
	if (codePoint <= 0x02ff) return codePoint !== 0x00d7 && codePoint !== 0x00f7;
	if (codePoint <= 0x1fff) return codePoint >= 0x0370 && codePoint !== 0x037e;
	if (codePoint <= 0x218f) {
		return (codePoint >= 0x200c && codePoint <= 0x200d) || codePoint >= 0x2070;
	}

	if (codePoint <= 0x2fef) return codePoint >= 0x2c00;
	if (codePoint <= 0xfaff) return codePoint >= 0xf900;
	if (codePoint <= 0xfdff) return codePoint >= 0xfc00;
	if (codePoint <= 0xfeff) return codePoint >= 0xfe70;
	if (codePoint <= 0xff5a) {
		return (codePoint >= 0xff21 && codePoint <= 0xff3a) || codePoint >= 0xff41;
	}

	return codePoint >= 0xff66 && codePoint <= 0xffdc;
}

export function isIdentifierPartCodePoint(codePoint: number): boolean {
	if (isIdentifierStartCodePoint(codePoint)) return true;
	if (codePoint >= 48 && codePoint <= 57) return true;
	if (codePoint === 0x20_0c || codePoint === 0x20_0d) return true;
	if (codePoint >= 0x03_00 && codePoint <= 0x03_61) return true;
	if (codePoint >= 0x20_30 && codePoint <= 0x20_4a) return true;
	return false;
}

export function isValidIdentifier(name: string): boolean {
	if (name.length === 0 || TYPESCRIPT_RESERVED_WORDS.has(name)) return false;

	const firstCodePoint = name.codePointAt(0);
	if (firstCodePoint === undefined || !isIdentifierStartCodePoint(firstCodePoint)) return false;

	let index = firstCodePoint > 0xff_ff ? 2 : 1;
	while (index < name.length) {
		const codePoint = name.codePointAt(index);
		if (codePoint === undefined || !isIdentifierPartCodePoint(codePoint)) return false;
		index += codePoint > 0xff_ff ? 2 : 1;
	}

	return true;
}
