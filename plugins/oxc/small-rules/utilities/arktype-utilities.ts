import { type } from "arktype";

import type { Type } from "arktype";

export function isSetOf<TValue>(matchType: Type<TValue>): Type<Set<TValue>> {
	return type(["instanceof", Set]).narrow((set, context): set is Set<TValue> => {
		for (const value of set) {
			const result = matchType(value);
			if (result instanceof type.errors) return context.reject(result.summary);
		}
		return true;
	});
}
