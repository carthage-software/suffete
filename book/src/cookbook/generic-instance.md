# Resolving a generic class against an instance

The analyser sees a method call on a generic class. The class is `Box<T>`; the method declares `function get(): T`; the instance is `Box<int>`. What is the return type of `$box->get()` ?

This is the canonical generic-resolution recipe.

## The PHP

```php
/**
 * @template T
 */
class Box<T> {
    /** @return T */
    public function get(): mixed { ... }
}

$box = new Box<int>();
$x = $box->get();   // type of $x is int
```

## The recipe

```rust,ignore
use suffete::{TypeId, ElementId, ElementKind, template::substitute};
use suffete::element::payload::{ObjectInfo, GenericParameterInfo, DefiningEntity};
use suffete::interner::interner;
use suffete::world::World;
use mago_atom::Atom;

fn resolve_method_return<W: World>(
    instance_type: TypeId,        // Box<int>
    method_name: Atom,            // "get"
    declared_return: TypeId,      // T (a free GenericParameter)
    world: &W,
) -> TypeId {
    // 1. Extract the class name and arguments from the instance.
    let i = interner();
    let view = instance_type.as_ref();
    let elements = view.elements;
    if elements.len() != 1 || elements[0].kind() != ElementKind::Object {
        // not a single-class type; analyser fallback (e.g. union of resolutions)
        return declared_return;
    }

    let obj_info: &ObjectInfo = i.get_object(elements[0]);
    let class_name = obj_info.name;
    let class_entity = i.intern_defining_entity(DefiningEntity::ClassLike(class_name));
    let actual_args = obj_info.type_args
        .map(|id| i.get_type_list(id).to_vec())
        .unwrap_or_default();

    // 2. Substitute the class's template parameters with the instance's args.
    substitute(declared_return, &|info: &GenericParameterInfo| -> Option<TypeId> {
        if info.defining_entity != class_entity {
            return None;
        }
        let position = world.template_parameter_index(class_name, info.name)?;
        actual_args.get(position).copied()
    })
}
```

## How it works

1. Pull the class name and actual arguments out of the instance type. (For a multi-class union, the analyser would resolve each branch and join.)
2. Walk the declared return type, replacing every `GenericParameter` Element whose `defining_entity` matches the class with the corresponding actual argument.
3. The walker handles deeply-nested uses of the parameter (e.g. `array<T, T>` with `T := int` becomes `array<int, int>`).

## Worked example

```rust,ignore
use suffete::{TypeBuilder, ElementId};
use suffete::element::payload::{GenericParameterInfo, DefiningEntity};
use suffete::interner::interner;
use mago_atom::atom;

let i = interner();
let box_class = i.intern_defining_entity(DefiningEntity::ClassLike(atom("Box")));

// Declared field type: T
let t_param: ElementId = i.intern_generic_parameter(GenericParameterInfo {
    name: atom("T"),
    defining_entity: box_class,
    constraint: suffete::prelude::TYPE_MIXED,
    qualifier: None,
});
let declared_return = TypeBuilder::new().push(t_param).build();

// Instance: Box<int>
let instance = TypeBuilder::new().push(
    ElementId::named_object_with_args(atom("Box"), &[suffete::prelude::TYPE_INT])
).build();

// Resolve.
let resolved = resolve_method_return(instance, atom("get"), declared_return, &world);
// resolved == int
```

## Multi-parameter case

```php
/**
 * @template K
 * @template V
 */
class Map {
    /**
     * @param K $key
     * @return V
     */
    public function get($key) { /* ... */ }
}
```

The recipe walks the entire declared return type. For `Map<string, int>::get`, the declared return is `V`, which substitutes to `int`. For a hypothetical `Map<string, int>::keys()` declaring `array<K>`, the return would substitute to `array<string>`.

The substitution is per-parameter; the position lookup via `template_parameter_index` handles each.

## Default-filled parameters

If the user wrote `Box` instead of `Box<int>`, the analyser fills `T` with the upper bound (`mixed` by default) and stamps the type with `from_template_default`. The recipe produces `mixed` as the resolved return; the variance check at the call site tolerates the default-fill (recording `CoercionCauses::TEMPLATE_DEFAULT` on the report).

## Multi-class union

```php
function f(Box<int>|Bag<string> $bw): mixed {
    return $bw->get();
}
```

The analyser resolves each branch separately and joins:

```rust,ignore
fn resolve_method_on_union<W: World>(
    instance_type: TypeId,
    method_name: Atom,
    world: &W,
) -> TypeId {
    let mut result = suffete::prelude::TYPE_NEVER;
    let opts = LatticeOptions::default();
    let mut report = LatticeReport::new();
    for &elem in instance_type.as_ref().elements {
        // Look up the method's declared return on this Element's class.
        let declared = world.method_return_type(elem, method_name);  // hypothetical
        let resolved = resolve_method_return(
            TypeBuilder::new().push(elem).build(),
            method_name,
            declared,
            world,
        );
        result = lattice::join(result, resolved, world, opts, &mut report);
    }
    result
}
```

The join folds the per-branch resolutions into the final return type.

## Inheritance

If `class IntBox extends Box<int>` and the analyser sees `(new IntBox())->get()`:

The recipe is unchanged ; the instance type is `IntBox`, not `Box`. To get the declared return for `get` (which lives on `Box`), the analyser looks up the method on the world ; the world walks the inheritance and returns `Box::get`'s declared return.

For substitution, the parameter is `T` on `Box` (not `T` on `IntBox`). The recipe's `class_entity` should be `Box`, not `IntBox`. The actual argument is found via `inherited_template_argument(IntBox, Box, 0) = int`.

A more robust recipe handles this:

```rust,ignore
fn resolve_method_return_with_inheritance<W: World>(
    instance_type: TypeId,
    method_name: Atom,
    declared_return: TypeId,
    declaring_class: Atom,        // Box (where get is declared)
    world: &W,
) -> TypeId {
    let i = interner();
    let view = instance_type.as_ref();
    let elements = view.elements;
    if elements.len() != 1 || elements[0].kind() != ElementKind::Object {
        return declared_return;
    }

    let obj_info: &ObjectInfo = i.get_object(elements[0]);
    let instance_class = obj_info.name;
    let actual_args = obj_info.type_args
        .map(|id| i.get_type_list(id).to_vec())
        .unwrap_or_default();

    // For each parameter declared on the declaring class, resolve via inheritance.
    let arity = world.template_parameter_arity(declaring_class);
    let mut bindings: Vec<Option<TypeId>> = Vec::with_capacity(arity);
    for pos in 0..arity {
        // The declaring class's parameter at position pos.
        // Find what the instance class supplies.
        if instance_class == declaring_class {
            bindings.push(actual_args.get(pos).copied());
        } else {
            let inherited = world.inherited_template_argument(instance_class, declaring_class, pos);
            // The inherited type may itself reference the instance class's parameters;
            // substitute those with actual_args.
            bindings.push(inherited.map(|t| substitute_class_params(t, instance_class, &actual_args, world)));
        }
    }

    let declaring_class_entity = i.intern_defining_entity(DefiningEntity::ClassLike(declaring_class));

    substitute(declared_return, &|info: &GenericParameterInfo| -> Option<TypeId> {
        if info.defining_entity != declaring_class_entity {
            return None;
        }
        let position = world.template_parameter_index(declaring_class, info.name)?;
        bindings.get(position).copied().flatten()
    })
}

fn substitute_class_params<W: World>(
    ty: TypeId,
    instance_class: Atom,
    actual_args: &[TypeId],
    world: &W,
) -> TypeId {
    let i = interner();
    let instance_entity = i.intern_defining_entity(DefiningEntity::ClassLike(instance_class));
    substitute(ty, &|info: &GenericParameterInfo| -> Option<TypeId> {
        if info.defining_entity != instance_entity {
            return None;
        }
        let position = world.template_parameter_index(instance_class, info.name)?;
        actual_args.get(position).copied()
    })
}
```

The lattice's [`specialise`](../generics/specialise.md) chapter is the formal protocol behind this recipe. The recipe above is what the analyser does to get the resolved return type.

## Performance

The substitution walker is O(tree size) per call. The world queries (`template_parameter_index`, `inherited_template_argument`) are typically O(1) amortised in a well-implemented analyser. Total cost per method-call resolution is sub-microsecond for typical analyser inputs.

> **See also:** [Substitute](../generics/substitute.md) for the substitution operation; [Specialise](../generics/specialise.md) for the inheritance protocol; [Templates in depth](../generics/templates.md) for the parameter-Element details.
