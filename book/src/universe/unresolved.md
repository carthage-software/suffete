# Unresolved elements

A handful of element kinds *name* a future resolution rather than carrying its result. They appear in the type universe because the analyser needs a way to express "the type that you'd get if you looked this up", and they get *replaced* by structural elements through the [expansion](../api/expand.md) operation when the lattice needs to reason about them.

If you call `refines(τ, σ, ...)` with an unresolved element on either side, the contract is that the analyser has already expanded it. The lattice itself does not invoke expansion; it would loop forever or worse if it did. The [expand](../api/expand.md) chapter has the rules for when and how the analyser does this.

| Form | PHP-side | Denotes |
|---|---|---|
| Alias | `/** @type UserId = int */` | A user-introduced name for a type. |
| Reference | `T` (a referenced template before binding) | A name that refers to some declared template parameter or alias. |
| Member reference | `Foo::T`, `self::T` | A reference to a class-member type. |
| Global reference | `T` declared at the file level | A reference to a global type variable. |
| Conditional | `T extends U ? X : Y` | A conditional type, resolved by checking subject vs target. |
| Derived | `key-of<T>`, `value-of<T>`, `T[K]`, ... | A type derived from another by a transformation. |
| Variable | analyser-introduced | A placeholder for an inferred-but-not-yet-pinned type. |

## Aliases

A user-introduced name for a type, declared at the package or class level:

```php
/** @type UserId = int */
```

Aliases are nominal: two distinct aliases with identical underlying types are *not* equal. `UserId` and `int` are not the same alias even when `UserId` is declared as `int`. Expansion turns them into the same structural type, but the alias preserves the name for diagnostics and refactoring.

## References

A *reference* is a name that has not yet been bound to a specific declaration. Used during PHPDoc parsing and resolution, before the analyser has determined whether the name refers to a class, an alias, a template parameter, or something else.

Expansion resolves the name through the analyser's symbol table:

- If the name is a class-like, becomes a named class.
- If the name is an alias, becomes the alias's underlying type.
- If the name is a template parameter, becomes a generic parameter.
- If the name is unknown, expansion fails and the analyser surfaces a diagnostic.

References can carry their own type arguments: `Foo<int>` is one reference. Intersections like `Foo<int> & Bar` use the [`Intersected`](./wrappers.md) wrapper.

## Member references

A reference to a type declared as a member of another type:

```php
/** @return self::ItemType */
```

Expansion looks up the name on the resolved class via the analyser's codebase model, substitutes the class's template environment, and returns the resulting type.

## Global references

A reference to a global type variable (rare, but used in some PHPDoc dialects):

```php
/** @global TItem $item */
```

Expansion looks the name up in the analyser's global type variable table.

## Conditional types

PHP-side: `T extends U ? X : Y`. Has four pieces: a **subject**, a **target**, a **then** branch, and an **otherwise** branch.

Expansion checks the `subject <: target` test under the current template environment. If true, the result is `then`; otherwise, the result is `otherwise`.

Conditional types are most useful inside generic declarations:

```php
/**
 * @template T
 * @return ($t extends int ? string : bool)
 */
function f($t) { ... }
```

After substitution `T := int(7)`, the conditional resolves to `string`.

## Derived types

A family of derived-type forms, each describing a transformation:

- **`key-of<T>`** — the keys of `T` if it's an array, list, or shape. `key-of<array{a: int, b: string}>` expands to `'a' | 'b'`.
- **`value-of<T>`** — the values of `T`. `value-of<array{a: int, b: string}>` expands to `int | string`.
- **`T[K]`** — index access; the type of accessing key `K` on `T`. Used for shape lookups.
- **`properties-of<T>`** — the properties of `T` as a structural shape.
- **`int-mask-of<T>` / `int-mask<L>`** — bitmask types, for flags.
- **`new<T>`** — the type of `new T(...)` ; useful when `T` is a class-string.
- **template-of-class** — the type of template parameter `P` on class `C` as instantiated on object `O`. Resolved by [specialise](../generics/specialise.md).

Expansion routes each variant to the matching transformation, which itself may produce more unresolved kinds (for example, `key-of<T>` where `T` is itself a reference).

## Inference variables

The analyser's own placeholder for an inferred-but-not-yet-pinned type. Variables are introduced by the analyser during inference and resolved by the analyser when inference completes. The lattice does not see them in finalised types.

## Why they exist in the universe

The alternative would be to keep unresolved forms as a separate AST and only build types for resolved cases. Suffete prefers a single universe for two reasons:

1. **Uniform structure.** Every type — resolved or not — is a finite union of elements. Walkers, transforms, and serialisation work on every type without a special case for "is this resolved?".
2. **Lazy expansion.** Resolving every reference at construction time would force the analyser to have a complete codebase model before constructing any type. The unresolved forms let the analyser construct types eagerly and resolve them on demand.

The trade-off is the rule above: lattice operations on unresolved forms are not directly defined. The analyser must expand first.

## A worked example

```php
/**
 * @template T
 * @param T $item
 * @return ($T extends list<infer U> ? U : T)
 */
function head($item) { ... }
```

The return type is a conditional with:

- `subject` = `T`
- `target` = `list<U>`
- `then` = `U`
- `otherwise` = `T`

Calling `head(['a', 'b'])` gives the analyser an argument type `list<string>`. Inference (see [standin](../generics/standin.md)) binds `T := list<string>`, and `U := string` is bound by the conditional's match.

Expansion then sees the `subject <: target` test holds and picks the `then` branch, yielding `string`.

Without expansion, the lattice cannot see any of this ; the conditional is opaque to it.

> **See also:** [expand](../api/expand.md) for the operation that resolves these; [generics](../generics/templates.md) for the template machinery.
