// oxlint-disable unicorn/prefer-code-point

const SPLIT_LOWER_TO_UPPER = /([\p{Ll}\d])(\p{Lu})/gu;
const SPLIT_UPPER_TO_UPPER = /(\p{Lu})(\p{Lu}\p{Ll})/gu;
const SPLIT_MARKER = "\0";
const SPLIT_REPLACE_VALUE = `$1${SPLIT_MARKER}$2`;

export function toPascalCase(value: string): string {
	const trimmed = value.trim();
	if (trimmed.length === 0) return "";

	const marked = trimmed
		.replace(SPLIT_LOWER_TO_UPPER, SPLIT_REPLACE_VALUE)
		.replace(SPLIT_UPPER_TO_UPPER, SPLIT_REPLACE_VALUE);

	let start = 0;
	let { length } = marked;

	while (marked.charCodeAt(start) === 0) start += 1;
	if (start === length) return "";

	while (marked.charCodeAt(length - 1) === 0) length -= 1;

	let result = "";
	let wordStart = start;

	for (let index = start; index <= length; index += 1) {
		if (index !== length && marked.charCodeAt(index) !== 0) continue;
		if (index > wordStart) {
			const word = marked.slice(wordStart, index);
			const firstByte = word.charCodeAt(0);

			if (result.length > 0 && firstByte >= 48 && firstByte <= 57) result += "_";
			// oxlint-disable-next-line typescript/no-non-null-assertion
			result += word[0]!.toUpperCase() + word.slice(1).toLowerCase();
		}
		wordStart = index + 1;
	}

	return result;
}
