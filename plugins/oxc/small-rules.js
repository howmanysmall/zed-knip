import { basename as e, dirname as t, extname as n, relative as r } from "node:path";
import { regex as a } from "arktype";
import { parseSync as i } from "oxc-parser";
import { ResolverFactory as o } from "oxc-resolver";
function s(e) {
	return e;
}
function c(e) {
	return e;
}
function l(e, t) {
	return e.type === `TSArrayType`
		? `Array<${l(e.elementType, t)}>`
		: e.type === `TSTypeOperator` && e.operator === `readonly` && e.typeAnnotation.type === `TSArrayType`
			? `ReadonlyArray<${l(e.typeAnnotation.elementType, t)}>`
			: t.getText(e);
}
function u({ parent: e }) {
	return !(
		(e.type === `TSRestType` && e.parent.type === `TSTupleType`) ||
		e.type === `TSTupleType` ||
		e.type === `TSArrayType` ||
		(e.type === `TSTypeOperator` && e.operator === `readonly`)
	);
}
const d = c({
	create(e) {
		function t(t) {
			e.report({
				fix(n) {
					return n.replaceText(t, l(t, e.sourceCode));
				},
				messageId: `useGenericArrayType`,
				node: t,
			});
		}
		return {
			TSArrayType(e) {
				u(e) && t(e);
			},
			TSTypeOperator(e) {
				e.operator !== `readonly` || e.typeAnnotation.type !== `TSArrayType` || (u(e) && t(e));
			},
		};
	},
	meta: {
		docs: { description: `Disallow bracket array type syntax and require Array<T> / ReadonlyArray<T>.` },
		fixable: `code`,
		messages: {
			useGenericArrayType: `Bracket array type syntax is not allowed. Use Array<T> or ReadonlyArray<T> generic syntax.`,
		},
		schema: [],
		type: `problem`,
	},
});
function f(e) {
	return typeof e == `object` && !!e && !Array.isArray(e);
}
function p(e) {
	return typeof e == `string` && e.length > 0;
}
function m(e) {
	if (!Array.isArray(e)) return !1;
	for (let t of e) if (typeof t != `string`) return !1;
	return !0;
}
function h(e) {
	if (!f(e)) return !1;
	for (let t of Object.values(e)) if (typeof t != `string`) return !1;
	return !0;
}
const g = new Map([[`omit`, { originalName: `Omit`, replacementName: `Except` }]]);
function _(e) {
	let t = new Map(g);
	if (!f(e) || !(`bannedTypes` in e)) return t;
	let { bannedTypes: n } = e;
	if (n === void 0) return t;
	if (m(n)) {
		for (let e of n) t.set(e.toLowerCase(), { originalName: e, replacementName: void 0 });
		return t;
	}
	if (h(n)) for (let [e, r] of Object.entries(n)) t.set(e.toLowerCase(), { originalName: e, replacementName: r });
	return t;
}
function ee(e) {
	if (e.type === `Identifier`) return e.name;
	if (e.type === `TSQualifiedName`) return e.right.name;
}
const te = c({
	create(e) {
		let [t] = e.options,
			n = _(t);
		return n.size === 0
			? {}
			: {
					TSTypeReference(t) {
						let r = ee(t.typeName);
						if (r === void 0) return;
						let i = n.get(r.toLowerCase());
						if (i !== void 0) {
							if (i.replacementName !== void 0 && i.replacementName !== ``) {
								e.report({
									data: { replacementName: i.replacementName, typeName: i.originalName },
									messageId: `bannedTypeWithReplacement`,
									node: t.typeName,
								});
								return;
							}
							e.report({ data: { typeName: i.originalName }, messageId: `bannedType`, node: t.typeName });
						}
					},
				};
	},
	meta: {
		docs: { description: `Ban configured TypeScript utility types, defaulting to Omit in favor of Except.` },
		messages: {
			bannedType: `Type '{{typeName}}' is banned by project configuration. Use the project-preferred alternative for this type.`,
			bannedTypeWithReplacement: `Type '{{typeName}}' is banned. Use '{{replacementName}}' instead.`,
		},
		schema: [
			{
				additionalProperties: !1,
				properties: {
					bannedTypes: {
						description: `Array of banned type names or an object mapping banned type names to preferred replacement names.`,
						oneOf: [
							{ items: { type: `string` }, type: `array` },
							{ additionalProperties: { type: `string` }, type: `object` },
						],
					},
				},
				type: `object`,
			},
		],
		type: `problem`,
	},
});
function v(e) {
	let t = e;
	for (;;)
		switch (t.type) {
			case `ChainExpression`:
			case `ParenthesizedExpression`:
			case `TSAsExpression`:
			case `TSInstantiationExpression`:
			case `TSNonNullExpression`:
			case `TSSatisfiesExpression`:
			case `TSTypeAssertion`:
				t = t.expression;
				break;
			default:
				return t;
		}
}
function y(e) {
	return e.computed
		? e.property.type === `Literal` && typeof e.property.value == `string`
			? e.property.value
			: void 0
		: e.property.type === `Identifier`
			? e.property.name
			: void 0;
}
function ne(e, t, n) {
	let r = e.getScope(t);
	for (; r !== null; ) {
		let e = r.set.get(n);
		if (e !== void 0 && e.defs.length > 0) return !0;
		r = r.upper;
	}
	return !1;
}
const re = new Set([`end`, `loc`, `parent`, `range`, `start`, `type`]);
function b(e) {
	return f(e) && typeof e.type == `string`;
}
function ie(e) {
	return re.has(e);
}
function x(e) {
	return e.type === `VariableDeclarator`;
}
function S(e) {
	return e.type === `Identifier` && typeof e.name == `string`;
}
function C(e) {
	return e.type === `Identifier`;
}
function ae(e) {
	return e.type === `JSXIdentifier` && `name` in e;
}
function oe(e) {
	return e.type === `ImportDeclaration`;
}
function w(e) {
	return e.type === `Literal` && typeof e.value == `string`;
}
function se(e) {
	return e.type === `CallExpression`;
}
function ce(e) {
	return e.type === `ImportSpecifier`;
}
function le(e) {
	return e.type === `ExportSpecifier`;
}
function T(e) {
	return e.type === `Property`;
}
function E(e) {
	return e.type === `MemberExpression`;
}
function ue(e) {
	return e.type === `AssignmentExpression`;
}
function de(e) {
	return e.type === `UnaryExpression`;
}
function fe(e) {
	return e.type === `BinaryExpression`;
}
function pe(e) {
	return e.type === `LogicalExpression`;
}
function me(e) {
	return e.type === `ConditionalExpression`;
}
function he(e) {
	return e.type === `SequenceExpression`;
}
function ge(e) {
	return e.type === `ObjectExpression`;
}
function _e(e) {
	return e.type === `MethodDefinition` || e.type === `TSAbstractMethodDefinition`;
}
function ve(e) {
	return e.type === `PropertyDefinition` || e.type === `TSAbstractPropertyDefinition`;
}
function ye(e) {
	return e.type === `ImportDefaultSpecifier`;
}
function be(e) {
	return e.type === `ImportNamespaceSpecifier`;
}
function xe(e) {
	return e.type === `VariableDeclaration`;
}
function Se(e) {
	return e.type === `ExportNamedDeclaration`;
}
function D(e) {
	return e.type === `FunctionDeclaration` || e.type === `FunctionExpression`;
}
function Ce(e) {
	return e.type === `ArrowFunctionExpression` || e.type === `FunctionExpression`;
}
function we(e) {
	return e.type === `ClassDeclaration` || e.type === `ClassExpression`;
}
function Te(e) {
	return e.type === `TSTypeAliasDeclaration`;
}
function O(e) {
	return e.type === `Literal`;
}
function Ee(e) {
	return e.type === `ArrowFunctionExpression`;
}
function De(e) {
	return D(e) || Ee(e);
}
function Oe(e) {
	return e.type === `TSQualifiedName`;
}
function ke(e) {
	return e.type === `Literal` && typeof e.value == `number`;
}
function Ae(e) {
	return e.type === `NewExpression`;
}
function je(e) {
	return e.type === `ArrayExpression`;
}
function Me(e) {
	return e.type === `ObjectExpression`;
}
function Ne(e) {
	return e.type === `TemplateLiteral`;
}
function Pe(e) {
	return e.type === `ExpressionStatement`;
}
function Fe(e) {
	return e.type === `TSTypeAssertion`;
}
function Ie(e) {
	return e.type === `TSAsExpression`;
}
function Le(e) {
	return e.type === `AssignmentPattern`;
}
function Re(e) {
	return e.type === `ThisExpression`;
}
function ze(e) {
	if (!se(e) || e.optional) return !1;
	let { callee: t } = e;
	if (!C(t) || t.name !== `require` || e.arguments.length !== 1) return !1;
	let [n] = e.arguments;
	return n !== void 0 && w(n);
}
function Be(e) {
	return e.type === `ExportNamedDeclaration`;
}
const Ve = { environment: `standard`, requireExplicitGenericOnNewArray: !0 };
function k(e) {
	return e.type === `Identifier`;
}
function He(e, t) {
	let n = v(t.callee);
	return !k(n) || n.name !== `Array` ? !1 : !ne(e, n, `Array`);
}
function A(e, t) {
	if (
		e.type !== `TSTypeReference` ||
		!k(e.typeName) ||
		(e.typeName.name !== `Array` && e.typeName.name !== `ReadonlyArray`) ||
		e.typeArguments?.params.length !== 1
	)
		return;
	let [n] = e.typeArguments.params;
	return n === void 0 ? void 0 : t.getText(n);
}
const Ue = /:\s*(Array<.+>|ReadonlyArray<.+>)\s*=/u;
function We(e) {
	return Ue.exec(e) !== null;
}
function Ge(e) {
	if (k(e) || e.type === `ArrayPattern` || e.type === `ObjectPattern`) return e.typeAnnotation ?? void 0;
}
function Ke(e, t) {
	let { parent: n } = e;
	if (x(n) && n.init === e) {
		let e = Ge(n.id);
		return e === void 0 ? !1 : A(e.typeAnnotation, t) !== void 0;
	}
	return Le(n) && n.right === e
		? We(t.getText(n))
		: ve(n) && n.value === e && n.typeAnnotation !== void 0 && n.typeAnnotation !== null
			? A(n.typeAnnotation.typeAnnotation, t) !== void 0
			: (Ie(n) && n.expression === e) || (Fe(n) && n.expression === e)
				? A(n.typeAnnotation, t) !== void 0
				: !1;
}
function qe(e) {
	if (e === void 0) return !1;
	let { typeAnnotation: t } = e;
	return t.type !== `TSTypeReference` || !k(t.typeName) ? !1 : t.typeName.name === `ReadonlyArray`;
}
function Je(e) {
	let t = v(e);
	return O(t) && `value` in t
		? typeof t.value != `number`
		: je(t) || Me(t) || Ee(t) || D(t) || we(t)
			? !0
			: Ne(t)
				? t.expressions.length === 0
				: de(t)
					? t.operator === `void` || (t.operator === `typeof` && !t.prefix)
					: !1;
}
function Ye(e) {
	return e.type === `Identifier` || e.type === `PrivateIdentifier` ? !0 : j(e);
}
function j(e) {
	let t = v(e);
	if (k(t) || Re(t)) return !0;
	if (E(t)) return t.optional || t.object.type === `Super` || !j(t.object) ? !1 : t.computed ? j(t.property) : !0;
	if (de(t)) return t.operator === `delete` ? !1 : j(t.argument);
	if (fe(t) || pe(t)) return j(t.left) && j(t.right);
	if (me(t)) return j(t.test) && j(t.consequent) && j(t.alternate);
	if (Ne(t)) {
		for (let e of t.expressions) if (!j(e)) return !1;
		return !0;
	}
	if (je(t)) {
		for (let e of t.elements) if (e !== null && (e.type === `SpreadElement` || !j(e))) return !1;
		return !0;
	}
	if (Me(t)) {
		for (let e of t.properties)
			if (
				e.type === `SpreadElement` ||
				e.kind !== `init` ||
				e.method ||
				(e.computed && !Ye(e.key)) ||
				!j(e.value)
			)
				return !1;
		return !0;
	}
	if (he(t)) {
		for (let e of t.expressions) if (!j(e)) return !1;
		return !0;
	}
	return O(t);
}
function M(e, t) {
	let n = v(e);
	if (
		!(!se(n) || n.optional) &&
		!(!E(n.callee) || n.callee.optional) &&
		!(!k(n.callee.object) || n.callee.object.name !== t)
	)
		return y(n.callee) === `push` ? n : void 0;
}
function Xe(e, t, n) {
	for (let r = t; r < e.length; r += 1) {
		let t = e[r];
		if (!(t === void 0 || !Pe(t)) && M(t.expression, n) !== void 0) return !0;
	}
	return !1;
}
function Ze(e, t) {
	let n = [],
		r = 0;
	for (let i of e) {
		if (i.type === `SpreadElement`) {
			n[r++] = `...${t.getText(i.argument)}`;
			continue;
		}
		n[r++] = t.getText(i);
	}
	return `[${n.join(`, `)}]`;
}
function Qe(e, t, n, r, i) {
	if (n.init === null || r.length === 0) return [];
	let [a] = r,
		o = r.at(-1);
	if (a === void 0 || o === void 0) return [];
	let [s] = a.range;
	for (; s > 0; ) {
		let e = t.text[s - 1];
		if (e === ` ` || e === `	`) {
			--s;
			continue;
		}
		e ===
			`
` && --s;
		break;
	}
	return [e.replaceText(n.init, i), e.removeRange([s, o.range[1]])];
}
const $e = c({
	create(e) {
		let t = e.options?.[0],
			n = typeof t == `object` && t ? { ...Ve, ...t } : { ...Ve },
			{ sourceCode: r } = e;
		function i(t) {
			for (let n = 0; n < t.length; n += 1) {
				let i = t[n];
				if (i === void 0 || !xe(i) || (i.kind !== `const` && i.kind !== `let`) || i.declarations.length !== 1)
					continue;
				let [a] = i.declarations;
				if (
					a === void 0 ||
					!k(a.id) ||
					a.init === null ||
					!Ae(a.init) ||
					!He(r, a.init) ||
					a.init.arguments.length > 0 ||
					qe(Ge(a.id))
				)
					continue;
				let o = a.id.name,
					s = [],
					c = [],
					l = !1,
					u = n + 1;
				for (; u < t.length; ) {
					let e = t[u];
					if (e === void 0 || !Pe(e)) break;
					let n = M(e.expression, o);
					if (n === void 0 || n.arguments.length === 0) break;
					s.push(e);
					for (let e of n.arguments) {
						if (e.type === `SpreadElement`) {
							((l = !0), c.push(`...${r.getText(e.argument)}`));
							continue;
						}
						c.push(r.getText(e));
					}
					u += 1;
				}
				if (s.length === 0 || Xe(t, u, o)) continue;
				let d = `[${c.join(`, `)}]`;
				if (
					!(
						l ||
						s.some((e) => {
							let t = M(e.expression, o);
							if (t === void 0) return !0;
							for (let e of t.arguments) if (e.type === `SpreadElement` || !j(e)) return !0;
							return !1;
						})
					)
				) {
					e.report({
						fix(e) {
							return Qe(e, r, a, s, d);
						},
						messageId: `collapseArrayPushInitialization`,
						node: i,
					});
					continue;
				}
				e.report({
					messageId: `collapseArrayPushInitialization`,
					node: i,
					suggest: [
						{
							fix(e) {
								return Qe(e, r, a, s, d);
							},
							messageId: `suggestCollapseArrayPushInitialization`,
						},
					],
				});
			}
		}
		return {
			BlockStatement(e) {
				i(e.body);
			},
			NewExpression(t) {
				if (!He(r, t)) return;
				if (t.arguments.length === 0) {
					if (
						!n.requireExplicitGenericOnNewArray ||
						(t.typeArguments !== void 0 && t.typeArguments !== null && t.typeArguments.params.length > 0) ||
						Ke(t, r)
					)
						return;
					e.report({ messageId: `requireExplicitGenericOnNewArray`, node: t });
					return;
				}
				if (t.arguments.length > 1) {
					let [i] = t.arguments;
					if (
						(i !== void 0 && i.type !== `SpreadElement` && n.environment === `roblox-ts` && !Je(i)) ||
						i === void 0
					)
						return;
					let a = Ze(t.arguments, r);
					if (!t.arguments.some((e) => e.type === `SpreadElement`)) {
						e.report({
							fix(e) {
								return e.replaceText(t, a);
							},
							messageId: `avoidConstructorEnumeration`,
							node: t,
						});
						return;
					}
					e.report({
						messageId: `avoidConstructorEnumeration`,
						node: t,
						suggest: [
							{
								fix(e) {
									return e.replaceText(t, a);
								},
								messageId: `suggestArrayLiteral`,
							},
						],
					});
					return;
				}
				let [i] = t.arguments;
				if (i === void 0) return;
				if (i.type === `SpreadElement`) {
					e.report({
						messageId: `avoidSingleArgumentConstructor`,
						node: t,
						suggest: [
							{
								fix(e) {
									return e.replaceText(t, `[...${r.getText(i.argument)}]`);
								},
								messageId: `suggestArrayLiteral`,
							},
						],
					});
					return;
				}
				if (!Je(i)) {
					if (
						n.environment === `roblox-ts` ||
						(t.typeArguments !== void 0 && t.typeArguments !== null && t.typeArguments.params.length > 0)
					)
						return;
					let a = r.getText(i);
					e.report({
						messageId: `avoidLengthConstructorInStandard`,
						node: t,
						suggest: [
							{
								fix(e) {
									return e.replaceText(t, `Array.from({ length: ${a} })`);
								},
								messageId: `suggestArrayFromLength`,
							},
						],
					});
					return;
				}
				let a = `[${r.getText(i)}]`;
				e.report({
					fix(e) {
						return e.replaceText(t, a);
					},
					messageId: `avoidSingleArgumentConstructor`,
					node: t,
				});
			},
			Program(e) {
				i(e.body);
			},
		};
	},
	meta: {
		docs: {
			description: `Disallow array constructor element forms and enforce roblox-ts-aware constructor patterns.`,
		},
		fixable: `code`,
		hasSuggestions: !0,
		messages: {
			avoidConstructorEnumeration: `Do not use Array constructor enumeration arguments. Use an array literal instead.`,
			avoidLengthConstructorInStandard: `Length-based Array constructor is not allowed in standard mode. Prefer Array.from({ length: n }).`,
			avoidSingleArgumentConstructor: `Single-argument Array constructor form is not allowed here. Use an array literal instead.`,
			collapseArrayPushInitialization: `Collapse new Array<T>() + consecutive .push(...) calls into a single array literal initializer.`,
			requireExplicitGenericOnNewArray: `new Array() must use an explicit generic argument or a contextual Array<T>/ReadonlyArray<T> annotation.`,
			suggestArrayFromLength: `Replace with Array.from({ length: value }).`,
			suggestArrayLiteral: `Replace constructor form with an array literal.`,
			suggestCollapseArrayPushInitialization: `Collapse constructor + push sequence into a single array literal initializer.`,
		},
		schema: [
			{
				additionalProperties: !1,
				properties: {
					environment: {
						default: `standard`,
						description: `Array constructor environment mode: 'roblox-ts' allows new Array(length); 'standard' reports it.`,
						enum: [`roblox-ts`, `standard`],
						type: `string`,
					},
					requireExplicitGenericOnNewArray: {
						default: !0,
						description: `When true, zero-argument new Array() requires explicit generic type arguments or contextual array typing.`,
						type: `boolean`,
					},
				},
				type: `object`,
			},
		],
		type: `problem`,
	},
});
function N(e) {
	return e.type !== `PrivateIdentifier`;
}
function P(e, t, n) {
	if (e.type !== t.type) return !1;
	switch (e.type) {
		case `CallExpression`:
			return t.type === `CallExpression` && n.getText(e) === n.getText(t);
		case `Identifier`:
			return t.type === `Identifier` && e.name === t.name;
		case `Literal`:
			return t.type === `Literal` && e.value === t.value && e.raw === t.raw;
		case `MemberExpression`:
			return t.type !== `MemberExpression` ||
				e.computed !== t.computed ||
				e.optional !== t.optional ||
				!P(e.object, t.object, n)
				? !1
				: e.computed
					? !N(e.property) || !N(t.property)
						? !1
						: P(e.property, t.property, n)
					: e.property.type === `PrivateIdentifier` || t.property.type === `PrivateIdentifier`
						? e.property.type === `PrivateIdentifier` &&
							t.property.type === `PrivateIdentifier` &&
							e.property.name === t.property.name
						: t.property.type === `Identifier` && e.property.name === t.property.name;
		case `Super`:
			return t.type === `Super`;
		case `ThisExpression`:
			return t.type === `ThisExpression`;
		default:
			return !1;
	}
}
function et(e) {
	switch (e.type) {
		case `Identifier`:
		case `Literal`:
		case `ThisExpression`:
			return !0;
		case `MemberExpression`:
			return e.optional || !F(e.object)
				? !1
				: e.computed
					? N(e.property)
						? et(e.property)
						: !1
					: e.property.type === `Identifier` || e.property.type === `PrivateIdentifier`;
		default:
			return !1;
	}
}
function F(e) {
	switch (e.type) {
		case `Identifier`:
		case `ThisExpression`:
			return !0;
		case `MemberExpression`:
			return e.optional || !F(e.object)
				? !1
				: e.computed
					? N(e.property)
						? et(e.property)
						: !1
					: e.property.type === `Identifier` || e.property.type === `PrivateIdentifier`;
		default:
			return !1;
	}
}
function tt(e) {
	return e.type !== `CallExpression` ||
		e.optional ||
		e.arguments.length > 0 ||
		e.callee.type !== `MemberExpression` ||
		e.callee.optional ||
		e.callee.computed ||
		e.callee.property.type !== `Identifier`
		? !1
		: e.callee.property.name === `size`;
}
function nt(e) {
	return typeof e != `object` || !e
		? !1
		: `allowAutofix` in e
			? e.allowAutofix === void 0 || typeof e.allowAutofix == `boolean`
			: !0;
}
const rt = c({
	create(e) {
		let [t] = e.options,
			n = nt(t) && t.allowAutofix === !0,
			{ sourceCode: r } = e;
		return {
			AssignmentExpression(t) {
				if (
					t.operator !== `=` ||
					t.left.type !== `MemberExpression` ||
					!t.left.computed ||
					!tt(t.left.property) ||
					!P(t.left.object, t.left.property.callee.object, r)
				)
					return;
				let i = t.parent.type === `ExpressionStatement` ? t.parent : void 0;
				if (!(n && i !== void 0 && F(t.left.object))) {
					e.report({ messageId: `usePush`, node: t });
					return;
				}
				let a = r.getText(t.left.object),
					o = r.getText(t.right);
				e.report({
					fix(e) {
						return e.replaceText(i, `${a}.push(${o});`);
					},
					messageId: `usePush`,
					node: t,
				});
			},
		};
	},
	meta: {
		docs: {
			description: `Disallow array append assignments using array[array.size()] = value and prefer push-based appends.`,
		},
		fixable: `code`,
		messages: { usePush: `Do not append with array[array.size()] = value. Use array.push(value) instead.` },
		schema: [
			{
				additionalProperties: !1,
				properties: { allowAutofix: { default: !1, type: `boolean` } },
				type: `object`,
			},
		],
		type: `problem`,
	},
});
function it(e, t) {
	let n = [e];
	for (; n.length > 0; ) {
		let e = n.pop();
		if (e === void 0) break;
		t(e);
		for (let t in e) {
			if (ie(t)) continue;
			let r = Reflect.get(e, t);
			if (!(typeof r != `object` || !r) && r !== e.parent) {
				if (Array.isArray(r)) {
					for (let e = r.length - 1; e >= 0; --e) {
						let t = r[e];
						b(t) && n.push(t);
					}
					continue;
				}
				b(r) && n.push(r);
			}
		}
	}
}
function I(e, t) {
	t(e);
	for (let n of Object.values(e)) {
		if (Array.isArray(n)) {
			for (let r of n) r !== e.parent && b(r) && I(r, t);
			continue;
		}
		n !== e.parent && b(n) && I(n, t);
	}
}
const at = new Set([`catch`, `finally`, `then`]);
function ot(e) {
	let t = new Set();
	for (let n of e.body)
		n.type !== `MethodDefinition` ||
			n.kind !== `method` ||
			!n.value.async ||
			(n.key.type === `Identifier` && t.add(n.key.name));
	return t;
}
function st(e) {
	return e.callee.type !== `MemberExpression` || e.callee.property.type !== `Identifier`
		? !1
		: at.has(e.callee.property.name);
}
function ct({ callee: e }) {
	return (e.type === `ArrowFunctionExpression` || e.type === `FunctionExpression`) && e.async;
}
function lt({ callee: e }, t) {
	if (!(e.type !== `MemberExpression` || e.object.type !== `ThisExpression` || e.property.type !== `Identifier`))
		return t.has(e.property.name) ? e.property.name : void 0;
}
function ut(e) {
	let { parent: t } = e;
	return t?.type !== `AssignmentExpression` || t.right !== e
		? !1
		: t.left.type === `MemberExpression` && t.left.object.type === `ThisExpression`;
}
function dt(e) {
	let { parent: t } = e;
	if (!(t?.type !== `VariableDeclarator` || t.init !== e)) return t.id.type === `Identifier` ? t.id.name : void 0;
}
function ft(e) {
	return e.type !== `ArrowFunctionExpression` && e.type !== `FunctionExpression`
		? !1
		: e.parent.type !== `CallExpression` || e.parent.callee !== e;
}
function pt(e, t) {
	let n = e;
	for (; n !== t; ) {
		let { parent: e } = n;
		if (e === null) return !1;
		if (ft(e)) return !0;
		n = e;
	}
	return !1;
}
function mt(e, t) {
	let n = lt(e, t);
	if (n === void 0 || ut(e)) return;
	if (e.parent.type === `ExpressionStatement`)
		return { data: { methodName: n }, messageId: `unhandledAsyncCall`, node: e };
	let r = dt(e);
	return r === void 0 ? void 0 : { data: { variableName: r }, messageId: `orphanedPromise`, node: e };
}
function ht(e, t) {
	let n = [],
		r = 0;
	function i(i) {
		if (i !== e && pt(i, e)) return;
		if (i.type === `AwaitExpression`) {
			n[r++] = { messageId: `awaitInConstructor`, node: i };
			return;
		}
		if (i.type !== `CallExpression`) return;
		(st(i) && (n[r++] = { messageId: `promiseChainInConstructor`, node: i }),
			ct(i) && (n[r++] = { messageId: `asyncIifeInConstructor`, node: i }));
		let a = mt(i, t);
		a !== void 0 && (n[r++] = a);
	}
	return (I(e, i), n);
}
const gt = c({
	create(e) {
		function t(t) {
			e.report({ data: t.data, messageId: t.messageId, node: t.node });
		}
		return {
			"MethodDefinition[kind='constructor']": function (e) {
				if (
					e.value.type !== `FunctionExpression` ||
					e.value.body?.type !== `BlockStatement` ||
					e.parent.type !== `ClassBody`
				)
					return;
				let n = ot(e.parent),
					r = e.value.body,
					i = ht(r, n);
				for (let e of i) t(e);
			},
		};
	},
	meta: {
		docs: {
			description: `Disallow asynchronous operations inside class constructors. Constructors return immediately, so async work causes race conditions, unhandled rejections, and incomplete object states.`,
		},
		messages: {
			asyncIifeInConstructor: `Refactor this asynchronous operation outside of the constructor. Async IIFEs create unhandled promises and incomplete object state.`,
			awaitInConstructor: `Refactor this asynchronous operation outside of the constructor. Using 'await' in a constructor causes the class to be instantiated before the async operation completes.`,
			orphanedPromise: `Refactor this asynchronous operation outside of the constructor. Promise assigned to '{{variableName}}' is never consumed - errors will be silently swallowed.`,
			promiseChainInConstructor: `Refactor this asynchronous operation outside of the constructor. Promise chains (.then/.catch/.finally) in constructors lead to race conditions.`,
			unhandledAsyncCall: `Refactor this asynchronous operation outside of the constructor. Calling async method '{{methodName}}' without handling its result creates uncontrolled async behavior.`,
		},
		schema: [],
		type: `problem`,
	},
});
function _t(e, t) {
	let n = e.scan(t);
	return n === 0 ? 0 : 1 - (1 - e.probability) ** n;
}
function vt(e, t) {
	let n = 0;
	for (let r of e) {
		let e = _t(r, t);
		n = 1 - (1 - n) * (1 - e);
	}
	return n;
}
function yt(e, t) {
	return vt(e, t) >= 0.9;
}
function bt(e, t) {
	return t.some((t) => yt(e, t));
}
function xt(e) {
	return {
		probability: e,
		scan(e) {
			for (let t = 0; t < e.length - 1; t += 1) {
				let n = e.charAt(t),
					r = e.charAt(t + 1);
				if (n === n.toLowerCase() && r === r.toUpperCase() && r !== r.toLowerCase()) return 1;
			}
			return 0;
		},
	};
}
const St = /[-/^$*+?.()|[\]{}]/gu,
	Ct = String.raw`\$&`,
	wt = /\s+/gu;
function Tt(e) {
	return e.replaceAll(St, Ct);
}
function Et(e, t) {
	let n = t.map((e) => (typeof e == `string` ? new RegExp(Tt(e), `gu`) : new RegExp(e.source, `gu`)));
	return {
		probability: e,
		scan(e) {
			let t = e.replace(wt, ``),
				r = 0;
			for (let e of n) {
				e.lastIndex = 0;
				let n = t.match(e);
				n && (r += n.length);
			}
			return r;
		},
	};
}
const Dt = /\s/v;
function Ot(e, t) {
	let n = new Set(t);
	return {
		probability: e,
		scan(e) {
			for (let t = e.length - 1; t >= 0; --t) {
				let r = e.charAt(t);
				if (n.has(r)) return 1;
				if (!Dt.test(r) && r !== `*` && r !== `/`) return 0;
			}
			return 0;
		},
	};
}
const kt = /[ \t(),{}]/u;
function At(e, t) {
	let n = new Set(t);
	return {
		probability: e,
		scan(e) {
			let t = e.split(kt),
				r = 0;
			for (let e of t) n.has(e) && (r += 1);
			return r;
		},
	};
}
const jt =
		`public.abstract.class.implements.extends.return.throw.private.protected.enum.continue.assert.boolean.this.instanceof.interface.static.void.super.true.case:.let.const.var.async.await.break.yield.typeof.import.export`.split(
			`.`,
		),
	Mt = [`++`, `||`, `&&`, `===`, `?.`, `??`],
	Nt = [
		`for(`,
		`if(`,
		`while(`,
		`catch(`,
		`switch(`,
		`try{`,
		`else{`,
		`this.`,
		`window.`,
		/;\s+\/\//u,
		`import '`,
		`import "`,
		`require(`,
	],
	Pt = [`}`, `;`, `{`];
function Ft() {
	return [Ot(0.95, Pt), At(0.7, Mt), At(0.3, jt), Et(0.95, Nt), xt(0.5)];
}
const It = new Set([`BreakStatement`, `ContinueStatement`, `LabeledStatement`]);
function Lt(e) {
	return It.has(e.type);
}
const Rt = Ft();
function zt(e, t, n) {
	let r = e.loc.start.line,
		i = t.loc.start.line;
	if (r + 1 !== i) return !1;
	let a = n.getTokenAfter(e);
	return a ? a.loc.start.line > i : !0;
}
function Bt(e, t) {
	let n = [],
		r = 0,
		i = [],
		a = 0;
	for (let o of e)
		if (o.type === `Block`)
			(a > 0 &&
				((n[r++] = {
					comments: i,
					value: i.map(({ value: e }) => e).join(`
`),
				}),
				(i = []),
				(a = 0)),
				(n[r++] = { comments: [o], value: o.value }));
		else if (a === 0) i[a++] = o;
		else {
			let e = i.at(-1);
			e && zt(e, o, t)
				? (i[a++] = o)
				: ((n[r++] = {
						comments: i,
						value: i.map(({ value: e }) => e).join(`
`),
					}),
					(i = [o]),
					(a = 1));
		}
	return (
		a > 0 &&
			(n[r] = {
				comments: i,
				value: i.map(({ value: e }) => e).join(`
`),
			}),
		n
	);
}
const Vt = /\{/gv,
	Ht = /\}/gv;
function Ut(e) {
	let t = (e.match(Vt) ?? []).length - (e.match(Ht) ?? []).length;
	return t > 0 ? e + `}`.repeat(t) : t < 0 ? `{`.repeat(-t) + e : e;
}
function Wt(e) {
	return bt(
		Rt,
		e.split(`
`),
	);
}
function Gt(e) {
	return e.type !== `ReturnStatement` && e.type !== `ThrowStatement` ? !1 : e.argument?.type === `Identifier`;
}
function Kt(e) {
	return e.type === `UnaryExpression` && (e.operator === `-` || e.operator === `+`);
}
function qt(e) {
	return e.type === `Literal` && (typeof e.value == `string` || typeof e.value == `number`);
}
function Jt(e) {
	return f(e) && typeof e.type == `string`;
}
function Yt(e) {
	let t = [],
		n = 0;
	for (let r of e) Jt(r) && (t[n++] = r);
	return t;
}
function Xt(e, t) {
	if (e.type !== `ExpressionStatement`) return !1;
	let { expression: n } = e;
	return n.type === `Identifier` || n.type === `SequenceExpression` || Kt(n) || qt(n) || !t.trimEnd().endsWith(`;`);
}
function Zt(e, t) {
	if (e.length !== 1) return !1;
	let n = e.at(0);
	return n ? Lt(n) || Gt(n) || Xt(n, t) : !1;
}
const Qt = [/A 'return' statement can only be used within a function body/v];
function $t(e) {
	for (let t of e) {
		let e = !1;
		for (let n of Qt)
			if (n.test(t.message)) {
				e = !0;
				break;
			}
		if (!e) return !1;
	}
	return !0;
}
function en(e) {
	return (e.errors.length === 0 || $t(e.errors)) && e.program.body.length > 0;
}
function tn(e, t) {
	let r = n(t),
		a = i(`file${r || `.js`}`, e);
	if (en(a)) return a;
	if (r !== `.tsx` && r !== `.jsx`) {
		let t = i(`file.tsx`, e);
		if (en(t)) return t;
	}
}
function nn(e, t) {
	if (!Wt(e)) return !1;
	let n = tn(e, t);
	return n ? !Zt(Yt(n.program.body), e) : !1;
}
const rn = c({
	create(e) {
		return {
			"Program:exit": function () {
				let t = Bt(e.sourceCode.getAllComments(), e.sourceCode);
				for (let n of t) {
					let t = n.value.trim();
					if (t === `}` || !nn(Ut(t), e.filename)) continue;
					let r = n.comments.at(0),
						i = n.comments.at(-1);
					!r ||
						!i ||
						e.report({
							loc: { end: i.loc.end, start: r.loc.start },
							messageId: `commentedCode`,
							suggest: [
								{
									desc: `Remove this commented out code`,
									fix(e) {
										return e.removeRange([r.range[0], i.range[1]]);
									},
								},
							],
						});
				}
			},
		};
	},
	meta: {
		docs: { description: `Disallow commented-out code`, recommended: !1 },
		hasSuggestions: !0,
		messages: {
			commentedCode: `Commented-out code creates confusion about intent and clutters the codebase. Version control preserves history, making dead code comments unnecessary. Delete the commented code entirely. If needed later, retrieve it from git history.`,
		},
		schema: [],
		type: `suggestion`,
	},
});
function L(e) {
	return { constant: !0, value: e };
}
function R() {
	return { constant: !1 };
}
function z(e) {
	return { constant: !0, value: e };
}
function B() {
	return { constant: !1 };
}
function an(e) {
	let t = new Set();
	if (!e?.loopExitCalls) return t;
	for (let n of e.loopExitCalls) p(n) && t.add(n);
	return t;
}
function on(e) {
	let t = v(e);
	if (t.type === `Identifier`) return t.name;
	if (t.type !== `MemberExpression`) return;
	let n = on(t.object);
	if (n === void 0 || n.length === 0) return;
	let r = y(t);
	if (!(r === void 0 || r.length === 0)) return `${n}.${r}`;
}
function sn(e, t) {
	if (t.size === 0) return !1;
	let n = on(e.callee);
	return n === void 0 || n.length === 0 ? !1 : t.has(n);
}
function V(e, t) {
	if (t.size === 0) return !1;
	let n = v(e);
	switch (n.type) {
		case `ArrayExpression`:
			for (let e of n.elements)
				if (e) {
					if (e.type === `SpreadElement`) {
						if (V(e.argument, t)) return !0;
						continue;
					}
					if (V(e, t)) return !0;
				}
			return !1;
		case `ArrowFunctionExpression`:
		case `ClassExpression`:
		case `FunctionExpression`:
			return !1;
		case `AssignmentExpression`:
			return V(n.right, t);
		case `AwaitExpression`:
			return V(n.argument, t);
		case `BinaryExpression`: {
			let { left: e } = n;
			return e.type !== `PrivateIdentifier` && V(e, t) ? !0 : V(n.right, t);
		}
		case `CallExpression`:
			if (sn(n, t) || V(n.callee, t)) return !0;
			for (let e of n.arguments) {
				if (e.type === `SpreadElement`) {
					if (V(e.argument, t)) return !0;
					continue;
				}
				if (V(e, t)) return !0;
			}
			return !1;
		case `ConditionalExpression`:
			return V(n.test, t) || V(n.consequent, t) || V(n.alternate, t);
		case `LogicalExpression`:
			return V(n.left, t) || V(n.right, t);
		case `MemberExpression`:
			return V(n.object, t) ? !0 : n.computed ? V(n.property, t) : !1;
		case `NewExpression`:
			if (V(n.callee, t)) return !0;
			for (let e of n.arguments) {
				if (e.type === `SpreadElement`) {
					if (V(e.argument, t)) return !0;
					continue;
				}
				if (V(e, t)) return !0;
			}
			return !1;
		case `SequenceExpression`:
			return n.expressions.some((e) => V(e, t));
		case `TaggedTemplateExpression`:
			return V(n.tag, t) ? !0 : n.quasi.expressions.some((e) => V(e, t));
		case `TemplateLiteral`:
			return n.expressions.some((e) => V(e, t));
		case `UnaryExpression`:
		case `UpdateExpression`:
			return V(n.argument, t);
		case `YieldExpression`:
			return n.argument ? V(n.argument, t) : !1;
		default:
			return !1;
	}
}
function H(e) {
	let t = v(e);
	switch (t.type) {
		case `ArrayExpression`:
			return L([]);
		case `ArrowFunctionExpression`:
		case `ClassExpression`:
		case `FunctionExpression`:
			return L(!0);
		case `Identifier`:
			return t.name === `undefined`
				? L(void 0)
				: t.name === `NaN`
					? L(NaN)
					: t.name === `Infinity`
						? L(1 / 0)
						: R();
		case `Literal`:
			return L(t.value);
		case `LogicalExpression`: {
			let e = H(t.left);
			return e.constant
				? t.operator === `&&`
					? e.value === !0
						? H(t.right)
						: L(e.value)
					: t.operator === `||`
						? e.value === !0
							? L(e.value)
							: H(t.right)
						: e.value === void 0
							? H(t.right)
							: L(e.value)
				: R();
		}
		case `ObjectExpression`:
			return L({});
		case `SequenceExpression`: {
			let e = t.expressions.at(-1);
			return e ? H(e) : R();
		}
		case `TemplateLiteral`:
			return t.expressions.length > 0 ? R() : t.quasis.length === 0 ? L(``) : L(t.quasis[0]?.value.cooked ?? ``);
		case `UnaryExpression`: {
			if (t.operator === `typeof`) return L(`string`);
			if (t.operator === `void`) return L(void 0);
			let e = H(t.argument);
			return e.constant
				? t.operator === `!`
					? L(!e.value)
					: t.operator === `+` && typeof e.value == `number`
						? L(e.value)
						: t.operator === `-` && typeof e.value == `number`
							? L(-e.value)
							: t.operator === `~` && typeof e.value == `number`
								? L(~e.value)
								: R()
				: R();
		}
		default:
			return R();
	}
}
function U(e) {
	let t = v(e);
	if (t.type === `ConditionalExpression`) {
		let e = U(t.test);
		if (e.constant) return U(e.value === !0 ? t.consequent : t.alternate);
		let n = U(t.consequent),
			r = U(t.alternate);
		return n.constant && r.constant && n.value === r.value ? n : B();
	}
	if (t.type === `LogicalExpression`) {
		let e = U(t.left);
		if (!e.constant) return B();
		if (t.operator === `&&`) return e.value ? U(t.right) : z(!1);
		if (t.operator === `||`) return e.value === !0 ? z(!0) : U(t.right);
		let n = H(t.left);
		return n.constant ? (n.value === void 0 ? U(t.right) : z(!!n.value)) : B();
	}
	if (t.type === `SequenceExpression`) {
		let e = t.expressions.at(-1);
		return e ? U(e) : B();
	}
	let n = H(t);
	return n.constant ? z(!!n.value) : B();
}
const cn = new Set([`DoWhileStatement`, `ForInStatement`, `ForOfStatement`, `ForStatement`, `WhileStatement`]);
function ln(e) {
	return cn.has(e.type);
}
const un = new Set([`ArrowFunctionExpression`, `FunctionDeclaration`, `FunctionExpression`]);
function dn(e) {
	return un.has(e.type);
}
function fn(e, t) {
	let n = t;
	for (; n !== null; ) {
		if (n.type === `LabeledStatement` && n.label.name === e) return n.body;
		if (n.type === `Program`) return;
		n = n.parent;
	}
}
function pn(e, t) {
	if (e.label) return fn(e.label.name, e.parent) === t;
	let n = e.parent;
	for (; n !== null; ) {
		if (n.type === `Program` || dn(n) || n.type === `SwitchStatement`) return !1;
		if (ln(n)) return n === t;
		n = n.parent;
	}
	return !1;
}
function mn(e, t) {
	return e
		? e.type === `VariableDeclaration`
			? e.declarations.some((e) => (e.init ? V(e.init, t) : !1))
			: V(e, t)
		: !1;
}
function hn(e, t) {
	switch (e.type) {
		case `DoWhileStatement`:
		case `WhileStatement`:
			return V(e.test, t);
		case `ForInStatement`:
		case `ForOfStatement`:
			return V(e.right, t);
		case `ForStatement`:
			return !!(mn(e.init, t) || (e.test && V(e.test, t)) || (e.update && V(e.update, t)));
		default:
			return !1;
	}
}
function W(e, t, n) {
	switch (e.type) {
		case `BlockStatement`:
			return e.body.some((e) => W(e, t, n));
		case `BreakStatement`:
			return pn(e, t);
		case `DoWhileStatement`:
		case `WhileStatement`:
			return V(e.test, n) ? !0 : W(e.body, t, n);
		case `ExpressionStatement`:
			return V(e.expression, n);
		case `ForInStatement`:
		case `ForOfStatement`:
			return V(e.right, n) ? !0 : W(e.body, t, n);
		case `ForStatement`:
			return mn(e.init, n) || (e.test && V(e.test, n)) || (e.update && V(e.update, n)) ? !0 : W(e.body, t, n);
		case `IfStatement`:
			return W(e.consequent, t, n) ? !0 : e.alternate ? W(e.alternate, t, n) : !1;
		case `LabeledStatement`:
			return W(e.body, t, n);
		case `ReturnStatement`:
			return !0;
		case `SwitchStatement`:
			return e.cases.some((e) => e.consequent.some((e) => W(e, t, n)));
		case `TryStatement`:
			return !!(
				W(e.block, t, n) ||
				(e.handler && W(e.handler.body, t, n)) ||
				(e.finalizer && W(e.finalizer, t, n))
			);
		case `VariableDeclaration`:
			return e.declarations.some((e) => (e.init ? V(e.init, n) : !1));
		case `WithStatement`:
			return V(e.object, n) ? !0 : W(e.body, t, n);
		default:
			return !1;
	}
}
function G(e, t, n) {
	return e.constant ? (e.value ? (hn(t, n) ? !1 : !W(t.body, t, n)) : !0) : !1;
}
const gn = c({
		create(e) {
			let t = e.options?.[0],
				n = an(typeof t == `object` && t ? t : void 0);
			function r(t) {
				U(t).constant && e.report({ messageId: `unexpected`, node: t });
			}
			return {
				ConditionalExpression(e) {
					r(e.test);
				},
				DoWhileStatement(t) {
					G(U(t.test), t, n) && e.report({ messageId: `unexpected`, node: t.test });
				},
				ForStatement(t) {
					t.test && G(U(t.test), t, n) && e.report({ messageId: `unexpected`, node: t.test });
				},
				IfStatement(e) {
					r(e.test);
				},
				WhileStatement(t) {
					G(U(t.test), t, n) && e.report({ messageId: `unexpected`, node: t.test });
				},
			};
		},
		meta: {
			docs: {
				description: `Disallow constant conditions, but allow constant loops that include loop exits such as break, return, or configured calls.`,
			},
			messages: { unexpected: `Unexpected constant condition.` },
			schema: [
				{
					additionalProperties: !1,
					properties: { loopExitCalls: { items: { minLength: 1, type: `string` }, type: `array` } },
					type: `object`,
				},
			],
			type: `problem`,
		},
	}),
	_n = { checkPrivate: !0, checkProtected: !0, checkPublic: !0 };
function vn(e) {
	return f(e)
		? {
				checkPrivate: typeof e.checkPrivate == `boolean` ? e.checkPrivate : !0,
				checkProtected: typeof e.checkProtected == `boolean` ? e.checkProtected : !0,
				checkPublic: typeof e.checkPublic == `boolean` ? e.checkPublic : !0,
			}
		: _n;
}
function yn(e, t) {
	if (e.static || e.kind !== `method`) return !1;
	let n = e.accessibility ?? `public`;
	return !(
		(n === `private` && !t.checkPrivate) ||
		(n === `protected` && !t.checkProtected) ||
		(n === `public` && !t.checkPublic)
	);
}
function K(e, t) {
	if (t.has(e)) return !1;
	if ((t.add(e), e.type === `ThisExpression` || e.type === `Super`)) return !0;
	if (!f(e)) return !1;
	for (let n in e) {
		if (!Object.hasOwn(e, n)) continue;
		let r = e[n];
		if (r !== void 0) {
			if (Array.isArray(r)) {
				for (let e of r) if (b(e) && K(e, t)) return !0;
				continue;
			}
			if (b(r) && K(r, t)) return !0;
		}
	}
	return !1;
}
function bn({ value: e }) {
	return e.type === `FunctionExpression` ? K(e, new WeakSet()) : !1;
}
function xn(e) {
	return e.key.type === `Identifier` ? e.key.name : `unknown`;
}
const Sn = c({
		create(e) {
			let t = vn(e.options[0]);
			return {
				MethodDefinition(n) {
					!yn(n, t) ||
						bn(n) ||
						e.report({ data: { methodName: xn(n) }, messageId: `noInstanceMethodWithoutThis`, node: n });
				},
			};
		},
		meta: {
			docs: {
				description: `Detect instance methods that do not use 'this' and suggest converting them to standalone functions for better performance in roblox-ts.`,
			},
			messages: {
				noInstanceMethodWithoutThis: `Method '{{methodName}}' does not use 'this' and creates unnecessary metatable overhead in roblox-ts. Convert it to a standalone function for better performance.`,
			},
			schema: [
				{
					additionalProperties: !1,
					properties: {
						checkPrivate: {
							default: !0,
							description: `Check private methods (default: true)`,
							type: `boolean`,
						},
						checkProtected: {
							default: !0,
							description: `Check protected methods (default: true)`,
							type: `boolean`,
						},
						checkPublic: {
							default: !0,
							description: `Check public methods (default: true)`,
							type: `boolean`,
						},
					},
					type: `object`,
				},
			],
			type: `problem`,
		},
	}),
	Cn = new RegExp(
		String.raw`(?:@(?:link|linkcode|linkplain|see)\s+\{?\w+\b\}?)|` +
			String.raw`(?:\{@(?:link|linkcode|linkplain|see)\s+\w+\b\})|` +
			String.raw`(?:[@{](?:type|typedef|param|returns?|template|augments|extends|implements)\s+[^}]*\b\w+\b)`,
		`u`,
	),
	wn = new RegExp(
		String.raw`(?:@(?:link|linkcode|linkplain|see)\s+\{?(\w+)\b\}?)|` +
			String.raw`(?:\{@(?:link|linkcode|linkplain|see)\s+(\w+)\b\})|` +
			String.raw`(?:[@{](?:type|typedef|param|returns?|template|augments|extends|implements)\s+[^}]*\b(\w+)\b)`,
		`gu`,
	);
function Tn(e) {
	return e.type === `ImportDefaultSpecifier` || e.type === `ImportNamespaceSpecifier` || e.type === `ImportSpecifier`;
}
function En(e) {
	let t = new Set();
	for (let n of e)
		if (!(n.type !== `Block` || !Cn.test(n.value))) {
			wn.lastIndex = 0;
			for (let e of n.value.matchAll(wn)) {
				let n = e[1] ?? e[2] ?? e[3];
				n !== void 0 && t.add(n);
			}
		}
	return t;
}
const Dn = c({
	create(e) {
		let { sourceCode: t } = e,
			n = e.options[0]?.checkJSDoc ?? !0,
			r = n ? En(t.getAllComments()) : new Set(),
			i = [],
			a;
		return {
			ImportDeclaration(e) {
				a ??= e;
				for (let t of e.specifiers) Tn(t) && i.push({ identifierName: t.local.name, parent: e, specifier: t });
			},
			"Program:exit": function () {
				if (a === void 0) return;
				let o = t.getScope(a);
				for (let { identifierName: a, parent: s, specifier: c } of i) {
					let i = o.set.get(a);
					(i !== void 0 && i.references.length > 0) ||
						(n && r.has(a)) ||
						e.report({
							data: { identifierName: a },
							fix(e) {
								if (s.specifiers.length === 1) return e.remove(s);
								let n = t.getTokenAfter(c);
								if (s.specifiers[0] === c && n?.value === `,`) {
									let r = t.getTokenBefore(c);
									if (r !== null)
										return [e.removeRange([r.range[1], c.range[0]]), e.remove(c), e.remove(n)];
								}
								if (n?.value === `,`) return e.removeRange([c.range[0], n.range[1]]);
								let r = t.getTokenBefore(c);
								return r?.value === `,` ? e.removeRange([r.range[0], c.range[1]]) : e.remove(c);
							},
							messageId: `unusedImport`,
							node: c,
						});
				}
			},
		};
	},
	meta: {
		docs: { description: `Disallow unused imports` },
		fixable: `code`,
		messages: { unusedImport: `Import '{{identifierName}}' is defined but never used.` },
		schema: [
			{
				additionalProperties: !1,
				properties: {
					checkJSDoc: {
						default: !0,
						description: `Check if imports are referenced in JSDoc comments`,
						type: `boolean`,
					},
				},
				type: `object`,
			},
		],
		type: `problem`,
	},
});
function On(e) {
	return !e.computed && q(e.value);
}
function q(e) {
	if (e === void 0) return !1;
	switch (e.type) {
		case `ArrayExpression`:
			return e.elements.every((e) => (e === null ? !0 : e.type === `SpreadElement` ? !1 : q(e)));
		case `CallExpression`:
			return e.callee.type === `MemberExpression` && q(e.callee.object);
		case `Literal`:
			return !0;
		case `MemberExpression`:
			return q(e.object);
		case `ObjectExpression`:
			return e.properties.every((e) => (e.type === `SpreadElement` ? !1 : On(e)));
		default:
			return !1;
	}
}
function kn(e) {
	let t = e;
	for (; t.type === `MemberExpression`; ) {
		if (t.computed && !O(t.property)) return !1;
		t = t.object;
	}
	return !0;
}
function An(e) {
	return (
		e.type === `MethodDefinition` &&
		e.kind === `constructor` &&
		e.key.type === `Identifier` &&
		e.key.name === `constructor`
	);
}
const jn = c({
	create(e) {
		let [t = `always`] = e.options;
		return t === `never`
			? {
					PropertyDefinition(t) {
						t.static || e.report({ messageId: `unexpectedClassProperty`, node: t });
					},
				}
			: {
					ClassDeclaration(t) {
						for (let n of t.body.body)
							if (!(!An(n) || n.value.body === null))
								for (let t of n.value.body.body) {
									if (t.type !== `ExpressionStatement`) continue;
									let { expression: n } = t;
									if (n.type !== `AssignmentExpression`) continue;
									let { left: r } = n;
									if (r.type !== `MemberExpression` || r.object.type !== `ThisExpression`) continue;
									let { property: i } = r;
									(i.type === `Identifier` || O(i)) &&
										q(n.right) &&
										kn(r) &&
										e.report({ messageId: `unexpectedAssignment`, node: n });
								}
					},
					ClassExpression(t) {
						for (let n of t.body.body)
							if (!(!An(n) || n.value.body === null))
								for (let t of n.value.body.body) {
									if (t.type !== `ExpressionStatement`) continue;
									let { expression: n } = t;
									if (n.type !== `AssignmentExpression`) continue;
									let { left: r } = n;
									if (r.type !== `MemberExpression` || r.object.type !== `ThisExpression`) continue;
									let { property: i } = r;
									(i.type === `Identifier` || O(i)) &&
										q(n.right) &&
										kn(r) &&
										e.report({ messageId: `unexpectedAssignment`, node: n });
								}
					},
				};
	},
	meta: {
		docs: { description: `Prefer class properties to assignment of literals in constructors.` },
		messages: {
			unexpectedAssignment: `Constructor assigns a literal value to this.property. Literals are static and known at class definition time. Move to a class property declaration: propertyName = value; at class level. This clarifies intent and reduces constructor complexity.`,
			unexpectedClassProperty: `Class property declarations are disabled by rule configuration (mode: 'never'). Move initialization into the constructor: this.propertyName = value; inside constructor().`,
		},
		schema: [{ enum: [`always`, `never`], type: `string` }],
		type: `suggestion`,
	},
});
function Mn(e) {
	return !f(e) || typeof e.maximumStatements != `number` ? 1 : e.maximumStatements;
}
function Nn(e) {
	return e.type === `IfStatement` && e.alternate === null;
}
function Pn(e, t) {
	return (e.type === `ExpressionStatement` && t === 0) || (e.type === `BlockStatement` && e.body.length > t);
}
function Fn(e, t) {
	if (e.body.length !== 1) return !1;
	let [n] = e.body;
	return n === void 0 || !Nn(n) ? !1 : Pn(n.consequent, t);
}
const In = c({
	create(e) {
		let t = Mn(e.options[0]);
		function n(n) {
			Fn(n, t) && e.report({ messageId: `preferEarlyReturn`, node: n });
		}
		return {
			ArrowFunctionExpression(e) {
				e.body.type === `BlockStatement` && n(e.body);
			},
			FunctionDeclaration(e) {
				e.body !== null && n(e.body);
			},
			FunctionExpression(e) {
				e.body !== null && n(e.body);
			},
		};
	},
	meta: {
		docs: { description: `Prefer early returns over full-body conditional wrapping.`, recommended: !0 },
		messages: {
			preferEarlyReturn: `Function body is wrapped in a single conditional without an else branch. This increases nesting depth and cognitive load. Invert the condition and return early: if (!condition) return; then place the main logic at the top level.`,
		},
		schema: [
			{
				additionalProperties: !1,
				properties: { maximumStatements: { default: 1, minimum: 0, type: `number` } },
				type: `object`,
			},
		],
		type: `suggestion`,
	},
});
function J(e) {
	return e.type === `Identifier` && (e.name === `it` || e.name === `test`);
}
function Ln(e) {
	if (e.callee.type !== `MemberExpression` || !J(e.callee.object)) return !1;
	let t = y(e.callee);
	return t === `only` || t === `skip`;
}
function Rn(e) {
	return e.callee.type !== `MemberExpression` || !J(e.callee.object) ? !1 : y(e.callee) === `each`;
}
function zn(e) {
	let t = e.arguments.at(-1);
	return t === void 0 || !De(t) ? void 0 : t;
}
function Bn(e) {
	return (
		e.type === `DoWhileStatement` ||
		e.type === `ForInStatement` ||
		e.type === `ForOfStatement` ||
		e.type === `ForStatement` ||
		e.type === `WhileStatement`
	);
}
function Vn(e, t) {
	return e.type === t.type && e.range[0] === t.range[0] && e.range[1] === t.range[1];
}
function Hn(e, t) {
	return {
		hasCallback: e.hasCallback || t.hasCallback,
		hasIndeterminate: e.hasIndeterminate || t.hasIndeterminate,
		hasLoop: e.hasLoop || t.hasLoop,
	};
}
function Un(e, t) {
	if (Vn(e, t)) return { hasCallback: !1, hasIndeterminate: !1, hasLoop: !1 };
	let n = {
		hasCallback: De(e),
		hasIndeterminate:
			De(e) ||
			Bn(e) ||
			e.type === `ConditionalExpression` ||
			e.type === `IfStatement` ||
			e.type === `SwitchCase` ||
			e.type === `TryStatement`,
		hasLoop: Bn(e),
	};
	return e.parent === null ? n : Hn(n, Un(e.parent, t));
}
function Wn(e) {
	return e.callee.type === `Identifier`
		? J(e.callee)
		: Ln(e)
			? !0
			: e.callee.type === `CallExpression`
				? Rn(e.callee)
				: !1;
}
function Gn(e) {
	return Wn(e) ? zn(e) : void 0;
}
function Kn(e, t = []) {
	return e.callee.type === `Identifier` && (e.callee.name === `expect` || t.includes(e.callee.name));
}
function qn({ callee: e }) {
	return e.type !== `MemberExpression` || e.object.type !== `Identifier` || e.object.name !== `expect`
		? !1
		: y(e) === `assertions`;
}
function Jn({ callee: e }) {
	return e.type !== `MemberExpression` || e.object.type !== `Identifier` || e.object.name !== `expect`
		? !1
		: y(e) === `hasAssertions`;
}
function Yn(e, t = []) {
	let n = 0,
		r = !1,
		i = !1,
		a = !1,
		o = 0;
	return (
		it(e, (s) => {
			if (s.type !== `CallExpression` || !Kn(s, t)) return;
			let c = Un(s.parent, e);
			if ((c.hasLoop && (i = !0), c.hasCallback && (r = !0), c.hasIndeterminate)) {
				((a = !0), (o += 1));
				return;
			}
			n += 1;
		}),
		{ deterministic: n, hasExpectInCallback: r, hasExpectInLoop: i, hasIndeterminate: a, indeterminate: o }
	);
}
function Xn(e) {
	return f(e)
		? {
				additionalExpectCallNames: Array.isArray(e.additionalExpectCallNames)
					? e.additionalExpectCallNames.filter((e) => typeof e == `string`)
					: [],
				onlyFunctionsWithAsyncKeyword: e.onlyFunctionsWithAsyncKeyword === !0,
				onlyFunctionsWithExpectInCallback: e.onlyFunctionsWithExpectInCallback === !0,
				onlyFunctionsWithExpectInLoop: e.onlyFunctionsWithExpectInLoop === !0,
			}
		: {
				additionalExpectCallNames: [],
				onlyFunctionsWithAsyncKeyword: !1,
				onlyFunctionsWithExpectInCallback: !1,
				onlyFunctionsWithExpectInLoop: !1,
			};
}
function Zn(e) {
	return e.onlyFunctionsWithAsyncKeyword || e.onlyFunctionsWithExpectInCallback || e.onlyFunctionsWithExpectInLoop;
}
function Qn(e) {
	return e.body ?? void 0;
}
function $n(e) {
	let t = Qn(e);
	return t?.type === `BlockStatement` ? t : void 0;
}
function er(e, t, n, r, i, a) {
	return n + r === 0
		? !1
		: !Zn(t) ||
				(t.onlyFunctionsWithAsyncKeyword && e.async) ||
				(t.onlyFunctionsWithExpectInCallback && i) ||
				(t.onlyFunctionsWithExpectInLoop && a);
}
function tr(e) {
	let t = $n(e);
	if (t === void 0) return;
	let [n] = t.body;
	if (!(n?.type !== `ExpressionStatement` || n.expression.type !== `CallExpression`)) return n.expression;
}
function nr(e, t, n, r, i) {
	let a = $n(t);
	if (a === void 0) {
		e.report({ messageId: `haveExpectAssertions`, node: n });
		return;
	}
	let [o] = a.body;
	if (i) {
		e.report({
			messageId: `haveExpectAssertions`,
			node: n,
			suggest: [
				{
					fix(e) {
						return o === void 0
							? e.insertTextAfterRange([a.range[0], a.range[0] + 1], ` expect.hasAssertions();`)
							: e.insertTextBefore(
									o,
									`expect.hasAssertions();
`,
								);
					},
					messageId: `suggestAddingHasAssertions`,
				},
			],
		});
		return;
	}
	e.report({
		messageId: `haveExpectAssertions`,
		node: n,
		suggest: [
			{
				data: { count: String(r) },
				fix(e) {
					return o === void 0
						? e.insertTextAfterRange([a.range[0], a.range[0] + 1], ` expect.assertions(${r});`)
						: e.insertTextBefore(o, `expect.assertions(${r});\n`);
				},
				messageId: `suggestAddingAssertions`,
			},
		],
	});
}
function rr(e, t, n, r) {
	if (Jn(t)) {
		t.arguments.length > 0 && e.report({ messageId: `hasAssertionsTakesNoArguments`, node: t });
		return;
	}
	if (!qn(t)) return;
	if (t.arguments.length !== 1) {
		e.report({ messageId: `assertionsRequiresOneArgument`, node: t });
		return;
	}
	let [i] = t.arguments;
	if (i === void 0 || i.type === `SpreadElement` || !ke(i)) {
		e.report({ messageId: `assertionsRequiresNumberArgument`, node: t });
		return;
	}
	!r &&
		n > 0 &&
		i.value !== n &&
		e.report({ data: { actual: String(n), expected: String(i.value) }, messageId: `wrongAssertionCount`, node: t });
}
const ir = c({
		create(e) {
			let t = Xn(e.options[0]);
			return {
				CallExpression(n) {
					if (!Wn(n)) return;
					let r = Gn(n);
					if (r === void 0) return;
					let i = Qn(r);
					if (i === void 0) return;
					let {
						deterministic: a,
						indeterminate: o,
						hasIndeterminate: s,
						hasExpectInCallback: c,
						hasExpectInLoop: l,
					} = Yn(i, t.additionalExpectCallNames);
					if (!er(r, t, a, o, c, l)) return;
					let u = tr(r);
					if (u !== void 0 && (qn(u) || Jn(u))) {
						rr(e, u, a, s);
						return;
					}
					nr(e, r, n, a, s);
				},
			};
		},
		meta: {
			docs: { description: `Enforce expect assertion guards in Jest tests.`, recommended: !0 },
			hasSuggestions: !0,
			messages: {
				assertionsRequiresNumberArgument: `This argument should be a number`,
				assertionsRequiresOneArgument: "`expect.assertions` expects a single argument of type number",
				hasAssertionsTakesNoArguments: "`expect.hasAssertions` expects no arguments",
				haveExpectAssertions:
					"Every test should have either `expect.assertions(<number of assertions>)` or `expect.hasAssertions()` as its first expression",
				suggestAddingAssertions: "Add `expect.assertions({{count}})`",
				suggestAddingHasAssertions: "Add `expect.hasAssertions()`",
				wrongAssertionCount: `Expected {{expected}} assertions, but test has {{actual}} expect calls`,
			},
			schema: [
				{
					additionalProperties: !1,
					properties: {
						additionalExpectCallNames: { items: { type: `string` }, type: `array` },
						onlyFunctionsWithAsyncKeyword: { type: `boolean` },
						onlyFunctionsWithExpectInCallback: { type: `boolean` },
						onlyFunctionsWithExpectInLoop: { type: `boolean` },
					},
					type: `object`,
				},
			],
			type: `suggestion`,
		},
	}),
	ar = /^[A-Z][A-Z0-9_]*$/v;
function or(e) {
	let { type: t } = e;
	if (t === `module` || t === `global`) return !0;
	if (e.upper?.type === `global`) {
		let { block: t } = e.upper;
		if (t.type === `Program` && t.sourceType === `script`) return !0;
	}
	return !1;
}
const sr = c({
		create(e) {
			let t = !1;
			return {
				VariableDeclaration(e) {
					t = e.kind === `const`;
				},
				"VariableDeclaration:exit": function () {
					t = !1;
				},
				VariableDeclarator(n) {
					let { id: r } = n;
					if (!(r.type !== `Identifier` || !ar.test(r.name))) {
						if (!t) {
							e.report({ messageId: `mustUseConst`, node: n });
							return;
						}
						or(e.sourceCode.getScope(n)) || e.report({ messageId: `mustBeModuleScope`, node: n });
					}
				},
			};
		},
		meta: {
			docs: {
				description:
					"Prefer that screaming snake case variables always be defined using `const`, and always appear at module scope.",
			},
			messages: {
				mustBeModuleScope: `You must place screaming snake case at module scope. If this is not meant to be a module-scoped variable, use camelcase instead.`,
				mustUseConst:
					"You must use `const` when defining screaming snake case variables. If this is not a constant, use camelcase instead.",
			},
			schema: [],
			type: `suggestion`,
		},
	}),
	cr = /[A-Z]+(?![a-z])|[A-Z]?[a-z]+|\d+/gv,
	lr = /^[^A-Za-z0-9]+|[^A-Za-z0-9]+$/gv,
	ur = /([a-z0-9])([A-Z])/gv,
	dr = /[_\-\s]+/gv;
function fr(e) {
	return e.replaceAll(lr, ``).replaceAll(ur, `$1 $2`).replaceAll(dr, ` `).match(cr) ?? [];
}
function pr(e) {
	let t = fr(e),
		n = ``;
	for (let e of t) e.length !== 0 && (n += `${e.slice(0, 1).toUpperCase()}${e.slice(1).toLowerCase()}`);
	return n;
}
const mr = /^\d/u;
function hr(e) {
	if (e.id.type === `Identifier`) return e.id.name;
	if (!(e.id.type !== `Literal` || typeof e.id.value != `string`)) return mr.test(e.id.value) ? void 0 : e.id.value;
}
const gr = c({
		create(e) {
			return {
				TSEnumDeclaration(t) {
					let n = t.id.name;
					pr(n) !== n && e.report({ data: { identifier: n }, messageId: `notPascalCase`, node: t.id });
				},
				TSEnumMember(t) {
					let n = hr(t);
					n === void 0 ||
						pr(n) === n ||
						e.report({ data: { identifier: n }, messageId: `notPascalCase`, node: t.id });
				},
			};
		},
		meta: {
			docs: { description: `Enforce Pascal case when naming enums.`, recommended: !0 },
			messages: {
				notPascalCase: `Enum '{{ identifier }}' uses non-standard casing. TypeScript convention requires PascalCase for enum names and members to distinguish them from variables (camelCase) and constants (UPPER_CASE). Rename to PascalCase: capitalize first letter of each word, no underscores.`,
			},
			schema: [],
			type: `suggestion`,
		},
	}),
	_r = new Set([
		`alumni`,
		`axes`,
		`cacti`,
		`children`,
		`criteria`,
		`data`,
		`dice`,
		`feet`,
		`fungi`,
		`geese`,
		`indices`,
		`matrices`,
		`media`,
		`men`,
		`mice`,
		`octopi`,
		`people`,
		`phenomena`,
		`teeth`,
		`vertices`,
		`women`,
	]),
	vr = new Set([
		`alias`,
		`analysis`,
		`axis`,
		`basis`,
		`business`,
		`class`,
		`crisis`,
		`glass`,
		`news`,
		`series`,
		`species`,
		`status`,
		`thesis`,
	]),
	yr = new Set([
		`args`,
		`components`,
		`controllers`,
		`dto`,
		`dtos`,
		`entries`,
		`enums`,
		`hooks`,
		`items`,
		`keys`,
		`models`,
		`options`,
		`orders`,
		`pages`,
		`parameters`,
		`params`,
		`props`,
		`repositories`,
		`services`,
		`settings`,
		`types`,
		`values`,
		`vo`,
		`vos`,
	]),
	br = /[A-Z]+(?![a-z])|[A-Z]?[a-z]+|\d+/gv,
	xr = /^[A-Z]{2,}[sS]?$/v;
function Sr(e) {
	let t = [],
		n = 0;
	for (let r of e.split(`_`)) {
		let e = r.match(br);
		if (e !== null) for (let r of e) t[n++] = r;
	}
	return t;
}
const Cr = /^\d+$/v;
function wr(e) {
	for (let t = e.length - 1; t >= 0; --t) {
		let n = e[t];
		if (!(n === void 0 || Cr.test(n))) return { lowercased: n.toLowerCase(), original: n };
	}
}
const Tr = /[sS]$/v,
	Er = /(?:ch|sh|x|z)es$/v;
function Dr(e, t) {
	return _r.has(e) || yr.has(e)
		? !0
		: vr.has(e)
			? !1
			: (xr.test(t) && Tr.test(t)) ||
				e.endsWith(`ies`) ||
				e.endsWith(`ves`) ||
				Er.test(e) ||
				(e.endsWith(`s`) && !e.endsWith(`ss`) && !e.endsWith(`us`) && !e.endsWith(`is`));
}
function Or(e) {
	if (xr.test(e) && Tr.test(e)) return !0;
	let t = wr(Sr(e));
	return t !== void 0 && Dr(t.lowercased, t.original);
}
const kr = c({
		create(e) {
			return {
				TSEnumDeclaration({ id: t }) {
					let { name: n } = t;
					Or(n) && e.report({ data: { name: n }, messageId: `notSingular`, node: t });
				},
			};
		},
		meta: {
			docs: { description: `Prefer singular naming for enums.`, recommended: !0 },
			messages: { notSingular: `Enum name "{{name}}" should be singular.` },
			schema: [],
			type: `suggestion`,
		},
	}),
	Ar = `replace`,
	jr = `suggestion`,
	Mr = `A more descriptive name will do too.`,
	Nr = { args: `parameters`, char: `character`, dt: `deltaTime`, plr: `player` },
	Pr = [`char`],
	Fr = {
		acc: { accumulator: !0 },
		arg: { argument: !0 },
		args: { arguments: !0 },
		arr: { array: !0 },
		attr: { attribute: !0 },
		attrs: { attributes: !0 },
		btn: { button: !0 },
		cb: { callback: !0 },
		char: { character: !0 },
		conf: { config: !0 },
		ctx: { context: !0 },
		cur: { current: !0 },
		curr: { current: !0 },
		db: { database: !0 },
		def: { defer: !0, deferred: !0, define: !0, definition: !0 },
		dest: { destination: !0 },
		dev: { development: !0 },
		dir: { direction: !0, directory: !0 },
		dirs: { directories: !0 },
		dist: { distance: !0 },
		doc: { document: !0 },
		docs: { documentation: !0, documents: !0 },
		dst: { daylightSavingTime: !0, destination: !0, distribution: !0 },
		dt: { dateTime: !0, deltaTime: !0 },
		e: { error: !0, event: !0 },
		el: { element: !0 },
		elem: { element: !0 },
		elems: { elements: !0 },
		env: { environment: !0 },
		envs: { environments: !0 },
		err: { error: !0 },
		ev: { event: !0 },
		evt: { event: !0 },
		ext: { extension: !0 },
		exts: { extensions: !0 },
		fn: { func: !0, function: !0 },
		func: { function: !0 },
		i: { index: !0 },
		idx: { index: !0 },
		impl: { implementation: !0 },
		j: { index: !0 },
		len: { length: !0 },
		lib: { library: !0 },
		mod: { module: !0 },
		msg: { message: !0 },
		num: { number: !0 },
		obj: { object: !0 },
		opts: { options: !0 },
		param: { parameter: !0 },
		params: { parameters: !0 },
		pkg: { package: !0 },
		plr: { player: !0 },
		prev: { previous: !0 },
		prod: { production: !0 },
		prop: { property: !0 },
		props: { properties: !0 },
		rel: { related: !0, relationship: !0, relative: !0 },
		req: { request: !0 },
		res: { resource: !0, response: !0, result: !0 },
		ret: { returnValue: !0 },
		retval: { returnValue: !0 },
		sep: { separator: !0 },
		src: { source: !0 },
		stdDev: { standardDeviation: !0 },
		str: { string: !0 },
		tbl: { table: !0 },
		temp: { temporary: !0 },
		tit: { title: !0 },
		tmp: { temporary: !0 },
		util: { utility: !0 },
		utils: { utilities: !0 },
		val: { value: !0 },
		var: { variable: !0 },
		vars: { variables: !0 },
		ver: { version: !0 },
	},
	Ir = {
		defaultProps: !0,
		devDependencies: !0,
		EmberENV: !0,
		getDerivedStateFromProps: !0,
		getInitialProps: !0,
		getServerSideProps: !0,
		getStaticProps: !0,
		iOS: !0,
		obj: !0,
		propTypes: !0,
		setupFilesAfterEnv: !0,
	},
	Lr = [`i18n`, `l10n`],
	Rr = /(?=[A-Z])|(?<=[_.-])/u,
	zr = new Set(
		`any.as.boolean.break.case.catch.class.const.constructor.continue.debugger.declare.default.delete.do.else.enum.export.extends.false.finally.for.from.function.get.if.implements.import.in.instanceof.interface.let.module.new.null.number.of.package.private.protected.public.require.return.set.static.string.super.switch.symbol.this.throw.true.try.type.typeof.var.void.while.with.yield`.split(
			`.`,
		),
	),
	Br = /^[A-Za-z]+$/u;
function Vr(e) {
	return (e >= 65 && e <= 90) || (e >= 97 && e <= 122) || e === 36 || e === 95;
}
function Hr(e) {
	return e < 192
		? Vr(e)
		: e >= 12289 && e <= 55295
			? !0
			: e <= 767
				? e !== 215 && e !== 247
				: e <= 8191
					? e >= 880 && e !== 894
					: e <= 8591
						? (e >= 8204 && e <= 8205) || e >= 8304
						: e <= 12271
							? e >= 11264
							: e <= 64255
								? e >= 63744
								: e <= 65023
									? e >= 64512
									: e <= 65279
										? e >= 65136
										: e <= 65370
											? (e >= 65313 && e <= 65338) || e >= 65345
											: e >= 65382 && e <= 65500;
}
function Ur(e) {
	return !!(
		Hr(e) ||
		(e >= 48 && e <= 57) ||
		e === 8204 ||
		e === 8205 ||
		(e >= 768 && e <= 865) ||
		(e >= 8240 && e <= 8266)
	);
}
function Wr(e) {
	if (e.length === 0 || zr.has(e)) return !1;
	let t = e.codePointAt(0);
	if (t === void 0 || !Hr(t)) return !1;
	let n = t > 65535 ? 2 : 1;
	for (; n < e.length; ) {
		let t = e.codePointAt(n);
		if (t === void 0 || !Ur(t)) return !1;
		n += t > 65535 ? 2 : 1;
	}
	return !0;
}
const Gr = /(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])|(?<=[a-zA-Z])(?=\d)|(?<=\d)(?=[a-zA-Z])/u,
	Kr = /[.+^${}()|[\]\\]/gu,
	qr = a(`^/(?<first>.+)/(?<second>[gimsuy]*)$`, `v`);
function Y(e) {
	return e === e.toUpperCase();
}
function Jr(e) {
	return Y(e.charAt(0));
}
function Yr(e) {
	return e.charAt(0).toUpperCase() + e.slice(1);
}
function Xr(e) {
	return e.charAt(0).toLowerCase() + e.slice(1);
}
function Zr(e) {
	return e.split(Gr);
}
function Qr(e) {
	let t = e.match(/\$(\d+)/gu);
	if (t === null) return 0;
	let n = 0;
	for (let e of t) {
		let t = Number.parseInt(e.slice(1), 10);
		t > n && (n = t);
	}
	return n;
}
function $r(e) {
	let t = Qr(e);
	if (t === 0) return [];
	let n = Array(t);
	for (let e = 1; e <= t; e += 1) n[e - 1] = RegExp(`\\$${e}`, `gu`);
	return n;
}
function ei(e, t) {
	if (e.startsWith(`/`)) {
		let n = qr.exec(e);
		if (n?.groups !== void 0)
			return {
				matcher: {
					original: e,
					pattern: RegExp(`^${n.groups.first}$`, n.groups.second),
					replacement: t,
					replacementPatterns: $r(t),
				},
				type: `pattern`,
			};
	}
	if (e.includes(`*`) || e.includes(`?`)) {
		let n = e
				.replaceAll(Kr, String.raw`\$&`)
				.replaceAll(`*`, `(.*)`)
				.replaceAll(`?`, `(.)`),
			r = 0,
			i = t.replaceAll(`*`, () => `$${++r}`);
		return {
			matcher: { original: e, pattern: RegExp(`^${n}$`, `u`), replacement: i, replacementPatterns: $r(i) },
			type: `pattern`,
		};
	}
	return { original: e, replacement: t, type: `exact` };
}
function ti(e, t) {
	let n = t.exactMatchers.get(e);
	if (n !== void 0) return { matchedWord: e, replacement: n, shorthand: e };
	for (let n of t.matchers) {
		let t = e.match(n.pattern);
		if (t === null) continue;
		let r = n.replacement,
			i = 1;
		for (let e of n.replacementPatterns) ((r = r.replaceAll(e, t[i] ?? ``)), (i += 1));
		return { matchedWord: e, replacement: r, shorthand: n.original };
	}
}
function ni(e, t) {
	if (t.ignoreExact.has(e)) return !0;
	for (let n of t.ignoreMatchers) if (n.pattern.test(e)) return !0;
	return !1;
}
function ri(e, t) {
	return e === `internal` || typeof e == `boolean` ? e : t;
}
function ii(e) {
	if (!f(e)) return;
	let t = {};
	for (let [n, r] of Object.entries(e)) typeof r == `boolean` && (t[n] = r);
	return t;
}
function X(e, t) {
	return typeof e == `boolean` ? e : t;
}
function ai(e) {
	let t = f(e) ? e : void 0,
		n = ii(t?.allowList),
		r = X(t?.extendDefaultAllowList, !0) ? { ...Ir, ...n } : (n ?? {});
	return new Map(Object.entries(r));
}
function oi(e) {
	if (!f(e)) return {};
	let t = {};
	for (let [n, r] of Object.entries(e)) {
		if (r === !1) {
			t[n] = !1;
			continue;
		}
		if (!f(r)) continue;
		let e = {};
		for (let [t, n] of Object.entries(r)) typeof n == `boolean` && (e[t] = n);
		t[n] = e;
	}
	return t;
}
function si(e) {
	let t = f(e) ? e : void 0,
		n = X(t?.extendDefaultReplacements, !0),
		r = oi(t?.replacements),
		i = new Set([...Object.keys(Fr), ...Object.keys(r)]),
		a = [];
	for (let e of i) {
		let t = r[e],
			i = n ? (Fr[e] ?? {}) : {},
			o = t === !1 ? {} : { ...i, ...t };
		a.push([e, new Map(Object.entries(o))]);
	}
	return new Map(a);
}
function ci(e) {
	let t = f(e) ? e : void 0,
		n = [];
	for (let e of Lr) n.push(e);
	if (Array.isArray(t?.ignore)) for (let e of t.ignore) (typeof e == `string` || e instanceof RegExp) && n.push(e);
	return n.map((e) => (e instanceof RegExp ? e : new RegExp(e, `u`)));
}
function li(e) {
	let t = f(e) ? e : void 0,
		n = new Set(Pr);
	if (!m(t?.allowPropertyAccess)) return n;
	for (let e of t.allowPropertyAccess) n.add(e);
	return n;
}
function ui(e) {
	let t = f(e) ? e : void 0,
		n = new Map(),
		r = new Set(),
		i = [],
		a = [],
		o = h(t?.shorthands) ? t.shorthands : {},
		s = { ...Nr, ...o };
	for (let [e, t] of Object.entries(s)) {
		let r = ei(e, t);
		r.type === `exact` ? n.set(r.original, r.replacement) : a.push(r.matcher);
	}
	if (m(t?.ignoreShorthands))
		for (let e of t.ignoreShorthands) {
			let t = ei(e, ``);
			t.type === `exact` ? r.add(t.original) : i.push(t.matcher);
		}
	return { exactMatchers: n, ignoreExact: r, ignoreMatchers: i, matchers: a };
}
function di(e) {
	let t = f(e) ? e : void 0;
	return {
		allowList: ai(t),
		allowPropertyAccess: li(t),
		checkDefaultAndNamespaceImports: ri(t?.checkDefaultAndNamespaceImports, `internal`),
		checkFilenames: X(t?.checkFilenames, !0),
		checkProperties: X(t?.checkProperties, !1),
		checkShorthandImports: ri(t?.checkShorthandImports, `internal`),
		checkShorthandProperties: X(t?.checkShorthandProperties, !0),
		checkVariables: X(t?.checkVariables, !0),
		ignore: ci(t),
		replacements: si(t),
		shorthandConfiguration: ui(t),
	};
}
function fi(e, t) {
	if (Y(e) || t.allowList.get(e) === !0) return [];
	let n = t.replacements.get(Xr(e)) ?? t.replacements.get(e) ?? t.replacements.get(Yr(e));
	if (n === void 0) return [];
	let r = Jr(e) ? Yr : Xr,
		i = [...n.keys()].filter((e) => n.get(e) === !0).map(r);
	return i.length > 0 ? i.toSorted() : [];
}
function pi(e, t) {
	if (e.length === 0) return;
	let n = Zr(e),
		r = [],
		i = !1;
	for (let e of n) {
		let n = ti(e, t);
		n !== void 0 && ((i = !0), r.push(n));
	}
	if (!i) return;
	let a = ``,
		o = 0;
	for (let e of n) {
		let t = r[o];
		if (t?.matchedWord === e) {
			((a += t.replacement), (o += 1));
			continue;
		}
		a += e;
	}
	return { matches: r, replaced: a };
}
function mi(e, t) {
	if (ni(e, t)) return !0;
	let n = Zr(e),
		r = !1;
	for (let e of n) {
		let n = ti(e, t);
		if (n !== void 0 && ((r = !0), !ni(n.matchedWord, t))) return !1;
	}
	return r;
}
function hi(e, t, n) {
	if (n.has(e)) return !0;
	for (let e of t.matches) if (!n.has(e.matchedWord)) return !1;
	return t.matches.length > 0;
}
function Z(e, t, n = 3) {
	if (Y(e) || t.allowList.get(e) === !0 || t.ignore.some((t) => t.test(e))) return { total: 0 };
	let r = pi(e, t.shorthandConfiguration);
	if (r !== void 0) return mi(e, t.shorthandConfiguration) ? { total: 0 } : { samples: [r.replaced], total: 1 };
	let i = fi(e, t);
	if (i.length > 0) return { samples: i.slice(0, n), total: i.length };
	let a = e.split(Rr).filter(Boolean),
		o = !1,
		s = [],
		c = 0;
	for (let e of a) {
		let n = fi(e, t);
		if (n.length > 0) {
			((o = !0), (s[c++] = n));
			continue;
		}
		s[c++] = [e];
	}
	if (!o) return { total: 0 };
	let l = s.reduce((e, t) => e * t.length, 1),
		u = Math.min(l, n),
		d = Array.from({ length: u }, (e, t) => {
			let n = t,
				r = [];
			for (let e = s.length - 1; e >= 0; --e) {
				let t = s[e] ?? [],
					i = n % t.length;
				n = (n - i) / t.length;
				let a = t[i];
				a !== void 0 && r.unshift(a);
			}
			return r;
		});
	for (let e of d)
		for (let t = e.length - 1; t > 0; --t) {
			let n = e[t] ?? ``;
			Br.test(n) && e[t - 1]?.endsWith(n) === !0 && e.splice(t, 1);
		}
	return { samples: d.map((e) => e.join(``)), total: l };
}
function gi(e, t) {
	let n = t.replacements.get(e);
	if (n === void 0) return !1;
	for (let e of n.values()) if (e) return !0;
	return !1;
}
function Q(e, t, n) {
	let { samples: r = [], total: i } = t;
	if (i === 1) return { data: { discouragedName: e, nameTypeText: n, replacement: r[0] ?? `` }, messageId: Ar };
	let a = r.map((e) => `\`${e}\``).join(`, `),
		o = i - r.length;
	return (
		o > 0 && (a += `, ... (${o > 99 ? `99+` : o} more omitted)`),
		{ data: { discouragedName: e, nameTypeText: n, replacementsText: a }, messageId: jr }
	);
}
function _i(e) {
	let t = [e],
		n = 1;
	for (let r of e.childScopes) {
		let e = _i(r);
		for (let r of e) t[n++] = r;
	}
	return t;
}
function vi(e, t) {
	let n = t;
	for (; n !== null; ) {
		let t = n.set.get(e);
		if (t !== void 0) return t;
		n = n.upper;
	}
}
function yi(e, t) {
	return !t.some((t) => vi(e, t) !== void 0);
}
function bi(e, t, n = () => !0) {
	let r = e;
	if (!(!Wr(r) && ((r = `${r}_`), !Wr(r)))) {
		for (; !yi(r, t) || !n(r, t); ) r = `${r}_`;
		return r;
	}
}
function xi(e) {
	let t = new Set();
	for (let n of e.identifiers) t.add(n);
	for (let { identifier: n } of e.references) t.add(n);
	return [...t];
}
function Si(e, t) {
	return e.range[0] === t.range[0] && e.range[1] === t.range[1];
}
function Ci(e) {
	let { parent: t } = e;
	return !ce(t) || t.local !== e ? !1 : Si(t.local, t.imported);
}
function wi(e) {
	let { parent: t } = e;
	return !le(t) || t.local !== e ? !1 : Si(t.local, t.exported);
}
function Ti(e) {
	if (!S(e)) return !1;
	let { parent: t } = e;
	return T(t) && t.shorthand && t.value === e;
}
function Ei(e) {
	if (!S(e)) return !1;
	let { parent: t } = e;
	if (!Le(t) || t.left !== e) return !1;
	let n = t.parent;
	return T(n) ? n.shorthand : !1;
}
function Di(e) {
	if (!S(e)) return !1;
	let { parent: t } = e;
	if ((ye(t) && t.local === e) || (be(t) && t.local === e)) return !0;
	if (ce(t) && t.local === e) {
		let { imported: e } = t;
		if (C(e) && e.name === `default`) return !0;
	}
	return x(t) && t.id === e && t.init !== null && ze(t.init);
}
function Oi(e) {
	if (!S(e)) return !1;
	let { parent: t } = e;
	if (x(t) && t.id === e) {
		let e = t.parent;
		return xe(e) ? Se(e.parent) : !1;
	}
	return (D(t) && t.id === e) || (we(t) && t.id === e) || (Te(t) && t.id === e) ? Se(t.parent) : !1;
}
function ki(e) {
	return xi(e).every((e) => !Oi(e) && !ae(e));
}
function Ai(e, t, n) {
	if (S(e))
		return Ti(e) || Ei(e)
			? n.replaceText(e, `${e.name}: ${t}`)
			: Ci(e)
				? n.replaceText(e, `${e.name} as ${t}`)
				: wi(e)
					? n.replaceText(e, `${t} as ${e.name}`)
					: n.replaceText(e, t);
}
function ji(e, t, n) {
	let r = [],
		i = 0;
	for (let a of xi(e)) {
		let e = Ai(a, t, n);
		e !== void 0 && (r[i++] = e);
	}
	return r;
}
function Mi(e) {
	if (!S(e)) return !1;
	let { parent: t } = e;
	if (E(t) && t.property === e && !t.computed) {
		let e = t.parent;
		if (ue(e) && e.left === t) return !0;
	}
	return (T(t) && t.key === e && !t.computed && !t.shorthand && ge(t.parent)) ||
		(le(t) && t.exported === e && t.local !== e)
		? !0
		: (_e(t) || ve(t)) && t.key === e && !t.computed;
}
function Ni(e) {
	if (!S(e)) return !1;
	let { parent: t } = e;
	return T(t) && t.key === e && !t.computed && !t.shorthand && ge(t.parent);
}
function Pi(e) {
	if (e.type === `ImportBinding`) {
		let { parent: t } = e;
		if (t !== null && oe(t) && w(t.source)) return t.source.value;
	}
	if (e.type === `Variable`) {
		let { node: t } = e;
		if (x(t) && t.init !== null && ze(t.init)) {
			let [e] = t.init.arguments;
			if (e !== void 0 && w(e)) return e.value;
		}
	}
}
function Fi(e) {
	let t = Pi(e);
	return t === void 0 ? !1 : !t.includes(`node_modules`) && (t.startsWith(`.`) || t.startsWith(`/`));
}
function Ii(e, t) {
	return e === !1 ? !1 : e === `internal` ? Fi(t) : !0;
}
function Li(e) {
	return e.defs.length === 1 ? e.defs[0]?.type === `ClassName` : !1;
}
const Ri = [
	{
		additionalProperties: !1,
		properties: {
			allowList: { additionalProperties: { type: `boolean` }, type: `object` },
			allowPropertyAccess: { items: { type: `string` }, type: `array` },
			checkDefaultAndNamespaceImports: { enum: [!1, !0, `internal`] },
			checkFilenames: { type: `boolean` },
			checkProperties: { type: `boolean` },
			checkShorthandImports: { enum: [!1, !0, `internal`] },
			checkShorthandProperties: { type: `boolean` },
			checkVariables: { type: `boolean` },
			extendDefaultAllowList: { type: `boolean` },
			extendDefaultReplacements: { type: `boolean` },
			ignore: { items: { oneOf: [{ type: `object` }, { type: `string` }] }, type: `array` },
			ignoreShorthands: { items: { type: `string` }, type: `array` },
			replacements: {
				additionalProperties: {
					oneOf: [{ enum: [!1] }, { additionalProperties: { type: `boolean` }, type: `object` }],
				},
				type: `object`,
			},
			shorthands: { additionalProperties: { type: `string` }, type: `object` },
		},
		type: `object`,
	},
];
function zi(e) {
	return function (t, n) {
		return n.every((n) => {
			let r = e.get(n);
			return r === void 0 || !r.has(t);
		});
	};
}
function Bi(e) {
	let { parent: t } = e;
	return (E(t) && t.property === e && !t.computed) || (Oe(t) && t.right === e);
}
function Vi(e, t, n, r) {
	let i = [];
	((i[0] = t), r({ ...Q(e.name, { samples: i, total: 1 }, n ? `property` : `variable`), node: e }));
}
function Hi(e, t, n) {
	return (Di(t) && !Ii(n.checkDefaultAndNamespaceImports, e)) || (Ci(t) && !Ii(n.checkShorthandImports, e))
		? !0
		: !n.checkShorthandProperties && Ti(t);
}
function Ui(e) {
	if (e.name !== `plr`) return;
	let [t] = e.defs;
	if (t?.type !== `Variable` || !x(t.node) || t.node.init === null) return;
	let { init: n } = t.node;
	if (
		E(n) &&
		!n.computed &&
		C(n.object) &&
		n.object.name === `Players` &&
		C(n.property) &&
		n.property.name === `LocalPlayer`
	)
		return `localPlayer`;
}
function Wi(e, t, n, r) {
	let i = [],
		a = 0,
		o = 0;
	for (let s of e) {
		let e = bi(s, t, n);
		if (e !== void 0) {
			if (e !== s && gi(s, r)) {
				o += 1;
				continue;
			}
			e.length > 0 && (i[a++] = e);
		}
	}
	return { droppedDiscouraged: o, safeSamples: i };
}
function Gi(e, t, n) {
	let r = e.type === `Variable` && x(e.node) && e.node.init === null,
		i = e.type === `Parameter` && t.scope.type === `function` && t.scope.block.type === `ArrowFunctionExpression`,
		a = r || i;
	return (e, t) => !(!n(e, t) || (a && e === `arguments`));
}
function Ki(e, t, n, r, i, a, o) {
	for (let e of i) (a.has(e) || a.set(e, new Set()), a.get(e)?.add(r));
	e({
		...t,
		fix(e) {
			return ji(n, r, e);
		},
		node: o,
	});
}
function qi(e, t, n, r, i) {
	let [a] = e.defs;
	if (a === void 0) return;
	let o = a.name;
	if (!C(o) || Hi(a, o, t)) return;
	let s = Gi(a, e, r),
		c = Ui(e),
		l = c === void 0 ? Z(e.name, t) : { samples: [c], total: 1 };
	if (l.total === 0 || !l.samples) return;
	let { references: u } = e,
		d = [...u.map((e) => e.from), e.scope],
		{ droppedDiscouraged: f, safeSamples: p } = Wi(l.samples, d, s, t),
		m = p.length > 0 ? p : l.samples,
		h = typeof l.samples.length == `number` && l.samples.length === l.total ? Math.max(0, l.total - f) : l.total,
		g = e.name === `fn` && h > 1 ? m.map((e) => (e === `function_` ? `function` : e)) : m,
		_ = Q(o.name, { samples: g, total: h }, `variable`);
	if (h === 1 && p.length === 1 && ki(e)) {
		let [t] = p;
		if (t !== void 0) {
			Ki(i, _, e, t, d, n, o);
			return;
		}
	}
	i({ ..._, node: o });
}
function Ji(e, t) {
	if (!Li(e)) {
		t(e);
		return;
	}
	if (e.scope.type === `class`) {
		let [n] = e.defs;
		if (n === void 0) {
			t(e);
			return;
		}
		let r = n.name;
		if (!C(r)) {
			t(e);
			return;
		}
		t(e);
	}
}
function Yi(e, t) {
	for (let n of _i(e)) for (let e of n.variables) Ji(e, t);
}
const Xi = c({
	create(e) {
		let t = di(e.options[0]),
			n = e.physicalFilename,
			r = new WeakMap(),
			i = zi(r),
			{ sourceCode: a } = e;
		function o(n) {
			qi(n, t, r, i, e.report);
		}
		return {
			Identifier(n) {
				if (!S(n) || n.name === `__proto__`) return;
				let r = pi(n.name, t.shorthandConfiguration),
					i = Mi(n),
					a = Bi(n);
				if (r !== void 0 && (i || a)) {
					if (
						mi(n.name, t.shorthandConfiguration) ||
						!t.checkShorthandProperties ||
						(a && hi(n.name, r, t.allowPropertyAccess))
					)
						return;
					Vi(n, r.replaced, !0, e.report);
					return;
				}
				if (!t.checkProperties) return;
				let o = Z(n.name, t);
				if (o.total === 0 || !i) return;
				let s = Q(n.name, o, `property`);
				if (o.total === 1 && o.samples && Ni(n)) {
					let [t] = o.samples,
						{ parent: r } = n;
					if (t !== void 0 && T(r) && w(r.value) && Wr(t)) {
						e.report({
							...s,
							fix(e) {
								return e.replaceText(n, t);
							},
							node: n,
						});
						return;
					}
				}
				e.report({ ...s, node: n });
			},
			JSXOpeningElement({ name: n }) {
				if (!t.checkVariables || !ae(n) || !Jr(n.name)) return;
				let r = Z(n.name, t);
				if (r.total === 0) return;
				let i = Q(n.name, r, `variable`);
				e.report({ ...i, node: n });
			},
			"Program:exit": function (r) {
				if (t.checkFilenames && n !== `<input>` && n !== `<text>`) {
					let i = Math.max(n.lastIndexOf(`/`), n.lastIndexOf(`\\`)),
						a = n.slice(i + 1),
						o = a.lastIndexOf(`.`),
						s = o === -1 ? `` : a.slice(o),
						c = Z(o === -1 ? a : a.slice(0, o), t);
					if (c.total > 0 && c.samples) {
						let t = c.samples.map((e) => `${e}${s}`);
						e.report({ ...Q(a, { samples: t, total: c.total }, `filename`), node: r });
					}
				}
				t.checkVariables && Yi(a.getScope(r), o);
			},
		};
	},
	meta: {
		docs: { description: `Prevent abbreviations.`, recommended: !1 },
		fixable: `code`,
		messages: {
			[Ar]: `The {{nameTypeText}} \`{{discouragedName}}\` should be named \`{{replacement}}\`. ${Mr}`,
			[jr]: `Please rename the {{nameTypeText}} \`{{discouragedName}}\`. Suggested names are: {{replacementsText}}. ${Mr}`,
		},
		schema: Ri,
		type: `suggestion`,
	},
});
function Zi(e) {
	return e.endsWith(`Async`);
}
function $(e, t) {
	Zi(t.name) || e.report({ messageId: `missingAsyncSuffix`, node: t });
}
function Qi(e, t) {
	t.id.type === `Identifier` &&
		((t.init?.type !== `ArrowFunctionExpression` && t.init?.type !== `FunctionExpression`) ||
			(t.init.async && $(e, t.id)));
}
function $i(e, t) {
	!t.value.async || t.key.type !== `Identifier` || $(e, t.key);
}
const ea = c({
	create(e) {
		return {
			FunctionDeclaration(t) {
				!t.async || t.id === null || $(e, t.id);
			},
			MethodDefinition(t) {
				$i(e, t);
			},
			Property(t) {
				!t.method ||
					t.value.type !== `FunctionExpression` ||
					!t.value.async ||
					(t.key.type === `Identifier` && $(e, t.key));
			},
			PropertyDefinition(t) {
				(t.value?.type !== `ArrowFunctionExpression` && t.value?.type !== `FunctionExpression`) ||
					!t.value.async ||
					t.key.type !== `Identifier` ||
					$(e, t.key);
			},
			VariableDeclarator(t) {
				Qi(e, t);
			},
		};
	},
	meta: {
		docs: { description: `Require async function names to end with Async.` },
		messages: { missingAsyncSuffix: `Async functions must have names that end with Async.` },
		schema: [],
		type: `problem`,
	},
});
function ta(e) {
	if (!f(e) || !(`classes` in e) || !f(e.classes)) return new Map();
	let { classes: t } = e,
		n = new Map();
	for (let [e, r] of Object.entries(t)) typeof r == `string` && n.set(e, r);
	return n;
}
function na(e) {
	return e?.type === `module` || e?.type === `global`;
}
function ra(e) {
	if (e.type === `ImportDefaultSpecifier`) return e.local.name;
	if (e.type === `ImportSpecifier`) {
		if (e.imported.type === `Identifier`) return e.imported.name;
		if (typeof e.imported.value == `string`) return e.imported.value;
	}
}
function ia(e, t, n) {
	if (e.callee.type === `Identifier`) {
		let r = t.get(e.callee.name);
		if (r === void 0) return;
		let i = n.get(r);
		return i === void 0 ? void 0 : { className: r, importSource: i };
	}
	if (e.callee.type === `MemberExpression` && e.callee.property.type === `Identifier`) {
		let t = e.callee.property.name,
			r = n.get(t);
		return r === void 0 ? void 0 : { className: t, importSource: r };
	}
}
function aa(e, t, n) {
	if (typeof e.source.value != `string`) return;
	let r = e.source.value;
	for (let [i, a] of t) if (a === r) for (let t of e.specifiers) ra(t) === i && n.set(t.local.name, i);
}
const oa = c({
	create(e) {
		let { sourceCode: t } = e,
			n = ta(e.options[0]);
		if (n.size === 0) return {};
		let r = new Map();
		return {
			ImportDeclaration(e) {
				aa(e, n, r);
			},
			NewExpression(i) {
				let a = ia(i, r, n);
				a !== void 0 &&
					(na(t.getScope(i)) ||
						e.report({
							data: { className: a.className, importSource: a.importSource },
							messageId: `mustBeModuleLevel`,
							node: i,
						}));
			},
		};
	},
	meta: {
		docs: { description: `Require configured classes to be instantiated at module level only.` },
		messages: {
			mustBeModuleLevel: `'{{className}}' from '{{importSource}}' must be instantiated at module level only.`,
		},
		schema: [
			{
				additionalProperties: !1,
				properties: { classes: { additionalProperties: { type: `string` }, type: `object` } },
				type: `object`,
			},
		],
		type: `problem`,
	},
});
function sa(e) {
	return f(e) && e.metric === `statements` ? `statements` : `lines`;
}
function ca(e, t) {
	let n = e.consequent.length;
	if (n === 0) return !1;
	let [r] = e.consequent;
	if (r === void 0 || (n === 1 && r.type === `BlockStatement`)) return !1;
	if (t === `statements`) return n > 1;
	let i = e.consequent[n - 1];
	return i === void 0 ? !1 : r.loc.start.line !== i.loc.end.line;
}
const la = c({
	create(e) {
		let t = sa(e.options[0]);
		return {
			SwitchCase(n) {
				if (!ca(n, t)) return;
				let [r] = n.consequent,
					i = n.consequent.at(-1);
				r === void 0 ||
					i === void 0 ||
					e.report({
						fix(e) {
							return [
								e.insertTextBefore(
									r,
									`{
`,
								),
								e.insertTextAfter(
									i,
									`
}`,
								),
							];
						},
						messageId: `wrapCaseBody`,
						node: r,
					});
			},
		};
	},
	meta: {
		docs: { description: `Require braces around switch case bodies that span multiple lines.` },
		fixable: `code`,
		messages: { wrapCaseBody: `Wrap this switch case body in braces.` },
		schema: [
			{
				additionalProperties: !1,
				properties: { metric: { default: `lines`, enum: [`lines`, `statements`], type: `string` } },
				type: `object`,
			},
		],
		type: `problem`,
	},
});
function ua(e) {
	return e.includes(`u`) || e.includes(`v`);
}
function da(e, t) {
	return e.type === `Identifier` && e.name === t;
}
function fa(e) {
	if (!(e.type !== `Literal` || typeof e.value != `string`)) return e.value;
}
function pa(e) {
	return e.type !== `SpreadElement`;
}
const ma = c({
		create(e) {
			return {
				CallExpression(t) {
					if (!da(t.callee, `regex`)) return;
					if (t.arguments.length < 2) {
						e.report({ messageId: `requireUnicodeFlag`, node: t });
						return;
					}
					let [, n] = t.arguments;
					if (n === void 0 || !pa(n)) return;
					let r = fa(n);
					r !== void 0 && (ua(r) || e.report({ messageId: `requireUnicodeFlag`, node: t }));
				},
			};
		},
		meta: {
			docs: { description: `Require the 'u' or 'v' unicode flag on arktype regex() calls.` },
			messages: {
				requireUnicodeFlag: `Missing the 'u' or 'v' unicode flag on this regex() call. Use the unicode flag to avoid silently creating invalid regex patterns.`,
			},
			schema: [],
			type: `problem`,
		},
	}),
	ha = /([\p{Ll}\d])(\p{Lu})/gu,
	ga = /(\p{Lu})(\p{Lu}\p{Ll})/gu,
	_a = `$1\0$2`;
function va(e) {
	let t = e.trim();
	if (t.length === 0) return ``;
	let n = t.replace(ha, _a).replace(ga, _a),
		r = 0,
		{ length: i } = n;
	for (; n.charCodeAt(r) === 0; ) r += 1;
	if (r === i) return ``;
	for (; n.charCodeAt(i - 1) === 0; ) --i;
	let a = ``,
		o = r;
	for (let e = r; e <= i; e += 1)
		if (!(e !== i && n.charCodeAt(e) !== 0)) {
			if (e > o) {
				let t = n.slice(o, e),
					r = t.charCodeAt(0);
				(a.length > 0 && r >= 48 && r <= 57 && (a += `_`),
					(a += t[0].toUpperCase() + t.slice(1).toLowerCase()));
			}
			o = e + 1;
		}
	return a;
}
const ya = new o({ extensions: [`.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.json`, `.node`] }),
	ba = { found: !1 };
function xa(e, n) {
	if (!e.startsWith(`.`)) return ba;
	let { path: r } = ya.sync(t(n), e);
	return r === void 0 || r === `` ? ba : { found: !0, path: r };
}
function Sa(e) {
	return e.split(`/`).filter((e) => !e.startsWith(`.`));
}
function Ca(e) {
	return e.split(`/`).filter((e) => e === `..`).length;
}
function wa(e) {
	return e.some((e) => e === va(e) && !e.includes(`.`));
}
function Ta(t) {
	return e(t, n(t)) === `index`;
}
function Ea(e) {
	if (!e.includes(`fixtures`)) return !1;
	let t = e.indexOf(`fixtures`);
	return !wa(e.slice(0, t));
}
const Da = c({
		create(e) {
			let t = e.options?.[0],
				{ allow: n = [], maxDepth: i = 1 } = typeof t == `object` && t ? t : { allow: [], maxDepth: 1 },
				a = n.map((e) => new RegExp(e, `v`));
			return {
				ImportDeclaration(t) {
					let n = t.source.value;
					if (typeof n != `string` || !n.startsWith(`.`) || a.some((e) => e.test(n))) return;
					let { filename: o } = e;
					if (o === ``) return;
					let s = xa(n, o);
					if (!s.found) return;
					let c = r(o, s.path),
						l = Sa(c);
					if ((Ca(c) > 1 || l.includes(`components`)) && wa(l) && l.length > i && !Ta(c) && !Ea(l)) {
						e.report({ messageId: `noReachingIntoComponent`, node: t });
						return;
					}
					l.includes(`components`) &&
						l.length > i + 1 &&
						!Ta(c) &&
						!Ea(l) &&
						e.report({ messageId: `noReachingIntoComponent`, node: t });
				},
			};
		},
		meta: {
			docs: { description: `Prevent module imports between components.` },
			messages: {
				noReachingIntoComponent: `Do not reach into an individual component's folder for nested modules. Import from the closest shared components folder instead.`,
			},
			schema: [
				{
					additionalProperties: !1,
					properties: { allow: { items: { type: `string` }, type: `array` }, maxDepth: { type: `integer` } },
					type: `object`,
				},
			],
			type: `problem`,
		},
	}),
	Oa = /^[A-Z][A-Z0-9_]*$/u,
	ka = [
		String.raw`^Array\b`,
		String.raw`^Object\b`,
		String.raw`^Map\b`,
		String.raw`^Set\b`,
		String.raw`^WeakMap\b`,
		String.raw`^WeakSet\b`,
	];
function Aa(e) {
	let t = [],
		n = [e];
	for (; n.length > 0; ) {
		let e = n.pop();
		if (e === void 0) break;
		t.push(e);
		for (let t of e.childScopes) n.push(t);
	}
	return t;
}
function ja(e) {
	return e.type === `Identifier`;
}
function Ma(e) {
	return e.type === `VariableDeclarator`;
}
function Na(e) {
	return e.type === `VariableDeclaration`;
}
function Pa(e) {
	return Ce(e) || e.type === `ClassExpression`;
}
function Fa(e, t, n) {
	if (
		e.type === `ArrayExpression` ||
		e.type === `ObjectExpression` ||
		e.type === `JSXElement` ||
		e.type === `JSXFragment`
	)
		return !0;
	if (e.type !== `CallExpression` && e.type !== `NewExpression`) return !1;
	let r = n.getText(e.callee);
	return t.some((e) => e.test(r));
}
function Ia(e) {
	return e.type === `Program` || e.type === `BlockStatement`;
}
function La(e) {
	return e.type === `Literal`;
}
function Ra(e, t) {
	let { parent: n } = e;
	if (!Ia(n)) return !1;
	let { body: r } = n;
	for (let n = 0; n < r.length; n += 1) if (r[n] === e) return r[n + 1] === t;
	return !1;
}
function za(e) {
	let t = e.parent,
		n = e;
	for (; t != null; ) {
		if (Ma(t) && t.init === n) return t;
		((n = t), (t = t.parent));
	}
}
const Ba = s({
	meta: { name: `small-rules` },
	rules: {
		"array-type-generic": d,
		"ban-types": te,
		"no-array-constructor-elements": $e,
		"no-array-size-assignment": rt,
		"no-async-constructor": gt,
		"no-commented-code": rn,
		"no-constant-condition-with-break": gn,
		"no-instance-methods-without-this": Sn,
		"no-unused-imports": Dn,
		"no-useless-constants": c({
			create(e) {
				let { sourceCode: t } = e,
					[n] = e.options,
					r = (n?.ignoreCallPatterns ?? ka).map((e) => new RegExp(e, `u`));
				function i(e) {
					if (t.getCommentsInside(e).length > 0) return !0;
					let n = e.loc.start.line,
						r = e.loc.end.line;
					for (let r of t.getCommentsBefore(e)) {
						let e = r.loc.end.line;
						if (e === n || e === n - 1) return !0;
					}
					for (let n of t.getCommentsAfter(e)) {
						let e = n.loc.start.line;
						if (e === r || e === r + 1) return !0;
					}
					return !1;
				}
				return {
					"Program:exit": function (n) {
						let a = Aa(t.getScope(n));
						for (let n of a)
							if (n.type !== `global`)
								for (let a of n.variables) {
									let { name: o } = a;
									if (!Oa.test(o)) continue;
									let [s] = a.defs;
									if (s?.type !== `Variable`) continue;
									let c = s.node;
									if (!Ma(c) || !ja(c.id) || c.init === null) continue;
									let l = s.parent;
									if (l === null || !Na(l) || l.kind !== `const` || l.declarations.length !== 1)
										continue;
									let u = l.parent;
									if (Be(u)) continue;
									let d = c.init;
									if (Pa(d) || Fa(d, r, t)) continue;
									let f,
										p = 0;
									for (let e of a.references) e.isReadOnly() && ((p += 1), (f = e));
									if (p !== 1 || f === void 0 || f.from !== n || t.getScope(f.identifier) !== n)
										continue;
									let m = f.identifier;
									if (!ja(m)) continue;
									let h = za(m);
									if (h === void 0) continue;
									let g = h.parent;
									if (!Na(g) || g.kind !== `const`) continue;
									if (!((Ra(l, g) || La(d)) && !i(l))) {
										e.report({ data: { name: o }, messageId: `uselessConstantNoFix`, node: c.id });
										continue;
									}
									let _ = t.getText(d);
									e.report({
										data: { name: o },
										fix(e) {
											let n = [];
											n.push(e.replaceText(m, _));
											let [r] = l.range;
											for (; r > 0; ) {
												let e = t.text[r - 1];
												if (e === ` ` || e === `	`) {
													--r;
													continue;
												}
												break;
											}
											let i = l.range[1];
											for (; i < t.text.length; ) {
												let e = t.text[i];
												if (
													e ===
														`
` ||
													e === `\r`
												) {
													i += 1;
													continue;
												}
												break;
											}
											return (n.push(e.removeRange([r, i])), n);
										},
										messageId: `uselessConstant`,
										node: c.id,
									});
								}
					},
				};
			},
			meta: {
				docs: { description: `Disallow constants that do not add value.`, recommended: !0 },
				fixable: `code`,
				messages: {
					uselessConstant: `Constant '{{name}}' is only referenced once in the same scope. Inline it directly, or move it to a higher scope if reference stability is needed.`,
					uselessConstantNoFix: `Constant '{{name}}' is only referenced once in the same scope. It cannot be auto-inlined because the declaration is not adjacent to its usage or has attached comments. Inline it manually, or move it to a higher scope if reference stability is needed.`,
				},
				schema: [
					{
						additionalProperties: !1,
						properties: {
							ignoreCallPatterns: { default: [...ka], items: { type: `string` }, type: `array` },
						},
						type: `object`,
					},
				],
				type: `problem`,
			},
		}),
		"prefer-class-properties": jn,
		"prefer-early-return": In,
		"prefer-expect-assertions": ir,
		"prefer-module-scope-constants": sr,
		"prefer-pascal-case-enums": gr,
		"prefer-singular-enums": kr,
		"prevent-abbreviations": Xi,
		"require-async-suffix": ea,
		"require-module-level-instantiation": oa,
		"require-switch-case-braces": la,
		"require-unicode-regex": ma,
		"strict-component-boundaries": Da,
	},
});
export { Ba as default };
