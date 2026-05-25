export function getEnumNames<EnumRecord extends Record<string, string>>(
	enumObject: EnumRecord,
): Array<keyof EnumRecord> {
	return Object.keys(enumObject).filter((key) => Object.hasOwn(enumObject, key)) as Array<keyof EnumRecord>;
}
