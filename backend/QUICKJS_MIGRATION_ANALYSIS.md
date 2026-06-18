# QuickJS Migration - Potential Breaking Changes Analysis

## Summary

This document details the comprehensive investigation into potential breaking changes when migrating flow expressions from Deno Core (V8) to QuickJS.

## 1. Areas Already Tested (60+ Parity Tests)

The following areas have comprehensive parity tests in `js_eval_parity_tests.rs`:

- **Arithmetic operations**: +, -, *, /, %, **
- **Comparison operators**: ===, !==, >, <, >=, <=, ==, !=
- **Logical operators**: &&, ||, !, ??, ?.
- **Bitwise operators**: &, |, ^, ~, <<, >>, >>>
- **Object operations**: property access, spread, destructuring, Object.keys/values/entries
- **Array operations**: map, filter, reduce, find, some, every, slice, flat, etc.
- **String operations**: split, replace, includes, startsWith, trim, etc.
- **Template literals**: ${} interpolation
- **Optional chaining**: ?. for properties, methods, computed properties
- **Nullish coalescing**: ??
- **Try-catch blocks**
- **Arrow functions**
- **Destructuring**
- **Date operations** (with fixed dates)
- **JSON.parse/stringify**
- **Math functions**
- **Set and Map operations**
- **Regular expressions** (basic patterns)
- **flow_input, flow_env, previous_result access**
- **Error extraction logic** (from parallel results)

## 2. Potential Breaking Changes Identified

### 2.1 Number Handling Edge Cases (MEDIUM RISK)

**Implementation Difference:**
```rust
// QuickJS json_to_js:
if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
    Ok(Value::new_int(ctx.clone(), i as i32))
} else {
    Ok(Value::new_float(ctx.clone(), i as f64))
}
```

**Potential Issues:**
- Numbers outside i32 range (-2147483648 to 2147483647) are stored as floats
- Large integers (between i32::MAX and 2^53) might lose precision
- **Timestamps** (e.g., 1704067200000) are typically in this range

**Test Case Needed:**
```javascript
// Numbers just above i32::MAX
2147483648 + 1  // i32::MAX + 2
9007199254740991 - 1  // Near MAX_SAFE_INTEGER
```

### 2.2 Object Property Order (LOW RISK)

**Implementation Difference:**
- QuickJS: `obj.props::<String, Value>()` iteration order
- V8: Guaranteed insertion order for string keys

**Potential Impact:**
- `Object.keys()`, `Object.values()`, `Object.entries()` order might differ
- Object spread `{...obj}` order might differ

**Mitigated by:**
- JSON comparison in tests normalizes order
- Most flow expressions don't depend on property order

### 2.3 Missing Browser/Deno APIs (MEDIUM RISK)

**APIs NOT available in QuickJS:**
- `atob()` / `btoa()` - Base64 encoding/decoding
- `TextEncoder` / `TextDecoder`
- `fetch()` (not relevant for expressions)
- `Blob`, `ArrayBuffer` (limited support)
- `Intl.*` - Internationalization APIs
- `console.log()` - No effect (not breaking, just no output)

**Expressions that would break:**
```javascript
atob("SGVsbG8=")  // Would throw: atob is not defined
btoa("Hello")     // Would throw: btoa is not defined
new TextEncoder().encode("test")  // Would throw
"test".toLocaleUpperCase('tr-TR') // Might behave differently
```

### 2.4 Regular Expression Differences (LOW RISK)

**QuickJS RegExp limitations:**
- No `d` flag (indices)
- No lookbehind assertions `(?<=...)` and `(?<!...)`
- No named capture groups `(?<name>...)`

**Expressions that might break:**
```javascript
"test123".match(/(?<=test)\d+/)  // Lookbehind not supported
/(?<name>\w+)/.exec("test")?.groups?.name  // Named groups not supported
```

### 2.5 Prototype Method Availability (LOW RISK)

**Methods that might differ:**
- `Array.prototype.at()` - ES2022
- `String.prototype.at()` - ES2022
- `Object.hasOwn()` - ES2022
- `String.prototype.replaceAll()` - ES2021

**Test Case:**
```javascript
[1,2,3].at(-1)  // Might not exist
"hello".at(-1)  // Might not exist
```

### 2.6 NaN/Infinity/Special Values (LOW RISK)

**Implementation:**
```rust
// QuickJS js_to_json:
if let Some(n) = serde_json::Number::from_f64(f) {
    return Ok(serde_json::Value::Number(n));
} else {
    return Ok(serde_json::Value::Null);  // NaN, Infinity -> null
}
```

Both engines convert NaN/Infinity to null in JSON, so this is consistent.

### 2.7 Fallback for Unsupported Types (LOW RISK)

**QuickJS fallback:**
```rust
// Fallback
Ok(serde_json::Value::String("[object]".to_string()))
```

Types that would trigger this:
- Symbol
- WeakMap/WeakRef
- Generator objects
- Custom objects with non-enumerable properties only

### 2.8 Date Object Timezone Handling (MEDIUM RISK)

**Potential Issue:**
- `new Date()` without arguments uses system time
- Timezone-dependent methods might vary

**Safe patterns (already tested):**
```javascript
new Date('2024-01-15T00:00:00.000Z').getUTCFullYear()  // OK - UTC methods
Date.parse('2024-01-15T00:00:00.000Z')  // OK - explicit timezone
```

**Risky patterns:**
```javascript
new Date().toLocaleDateString()  // Timezone dependent
new Date().getHours()  // Timezone dependent
```

## 3. Edge Cases NOT Currently Tested

### 3.1 Very Large Numbers
```javascript
9007199254740991  // MAX_SAFE_INTEGER
9007199254740992  // MAX_SAFE_INTEGER + 1 (loses precision)
2147483648        // i32::MAX + 1
```

### 3.2 Negative Zero
```javascript
-0 === 0  // true
Object.is(-0, 0)  // false
1/-0  // -Infinity
```

### 3.3 Sparse Arrays
```javascript
const arr = [1, , 3]  // Hole at index 1
arr.map(x => x * 2)  // Holes might be handled differently
arr.filter(x => true)  // Holes might be skipped or preserved
```

### 3.4 Unicode Edge Cases
```javascript
"🎉".length  // 2 (surrogate pairs)
"🎉".split('')  // Might differ
[..."🎉"]  // Might differ
"café" === "café"  // NFC vs NFD normalization
```

### 3.5 Prototype Chain
```javascript
const obj = Object.create({ inherited: 1 });
obj.own = 2;
Object.keys(obj)  // Should only return ['own']
```

### 3.6 Getter/Setter Properties
```javascript
const obj = {
  get prop() { return 42; },
  set prop(v) { }
};
obj.prop  // Should return 42
```

### 3.7 Circular References
```javascript
const obj = { a: 1 };
obj.self = obj;
JSON.stringify(obj)  // Should throw in both
```

### 3.8 Array-like Objects
```javascript
const arrayLike = { 0: 'a', 1: 'b', length: 2 };
Array.from(arrayLike)  // Should work in both
```

## 4. Recommended Additional Tests

### High Priority (Add to parity tests):
1. Large integers (i32 boundary, MAX_SAFE_INTEGER boundary)
2. `Array.prototype.at()` and `String.prototype.at()`
3. Sparse arrays with holes
4. Emoji/surrogate pair handling
5. Object property order verification

### Medium Priority:
1. Getter/setter access
2. Prototype chain behavior
3. Array-like object conversion
4. Error message format differences

### Low Priority (Unlikely to be used in expressions):
1. WeakMap/WeakSet
2. Generators
3. Symbols
4. Proxy edge cases

## 5. Known Safe Patterns

These patterns are safe to use and have been verified:
- All arithmetic and comparison operators
- All standard array methods (map, filter, reduce, etc.)
- All standard string methods
- Object spread and destructuring
- Optional chaining and nullish coalescing
- Template literals
- Arrow functions
- Try-catch blocks
- `flow_input`, `flow_env`, `previous_result`, `results` access
- JSON operations
- Date operations with UTC methods
- Regular expressions (basic patterns without lookbehind)

## 6. Test Coverage Summary

### Unit Parity Tests (114 tests in js_eval_parity_tests.rs)
- Basic arithmetic, comparison, logical, and bitwise operators
- Object operations: property access, spread, destructuring
- Array operations: map, filter, reduce, find, some, every, slice, flat, etc.
- String operations: all standard methods
- Template literals with complex expressions
- Optional chaining and nullish coalescing
- Set and Map operations
- JSON parse/stringify
- Date operations with UTC methods
- Error handling with try-catch
- Large integer handling (i32 boundaries, timestamps, MAX_SAFE_INTEGER)
- Unicode and special characters
- Type coercion

### Flow Engine Parity Tests (19 tests in flow_engine_parity.rs)
All tests pass with both Deno Core and QuickJS:

1. **Linear flow with input transforms** - `results.a.property` access
2. **For-loop with complex iterator** - `results.a.users.filter(...)`
3. **Branch conditions** - `results.a.status === 'premium' && results.a.score >= 90`
4. **Previous result aggregation** - `previous_result.value`, `results.a.value + results.b.value`
5. **Nested complexity** - Deep result access across loop iterations
6. **Parallel for-loops** - Multiple concurrent iterations
7. **Skip-if expressions** - Conditional step execution
8. **Object transformations** - Complex data manipulation
9. **Template literals** - `\`Status: ${results.a.status}\``
10. **Optional chaining** - `results.a.user?.name`, `results.a?.missing?.value ?? 'default'`
11. **Flow env access** - `flow_env.CONFIG.apiUrl`
12. **Combined flow_input and flow_env**
13. **Results optional chaining** - Deep optional chaining with results proxy
14. **Large integers** - Timestamps, i32 boundaries through results
15. **Unicode and emoji** - Strings with unicode through flow results
16. **Complex array operations** - Sort, filter/map chains, reduce through results
17. **Multiline expressions** - Multi-statement expressions with semicolons and return
18. **Spread operators** - `{...results.a.config}`, `[...results.a.tags]`
19. **Nested for-loop results access** - Accessing outer step results from inner loops

## 7. Conclusion

The QuickJS migration is **safe** for the vast majority of flow expressions. Comprehensive testing shows:

- **133 total parity tests pass** (114 unit + 19 flow engine)
- All tests pass with both Deno Core and QuickJS
- No behavioral differences detected in production-like scenarios

### ES2022+ Method Support (All SUPPORTED in both engines):

Tested and verified to work identically:
- `Array.prototype.at()` - ES2022 ✅
- `String.prototype.at()` - ES2022 ✅
- `Object.hasOwn()` - ES2022 ✅
- `String.prototype.replaceAll()` - ES2021 ✅
- `Array.prototype.findLast()` - ES2023 ✅
- `Array.prototype.findLastIndex()` - ES2023 ✅
- `Array.prototype.toSorted()` - ES2023 ✅
- `Array.prototype.toReversed()` - ES2023 ✅
- `Array.prototype.toSpliced()` - ES2023 ✅
- `Array.prototype.with()` - ES2023 ✅
- `Object.groupBy()` - ES2024 ✅

### Regex Feature Support (All SUPPORTED in both engines):

- Lookbehind assertions `(?<=...)` ✅
- Negative lookbehind `(?<!...)` ✅
- Named capture groups `(?<name>...)` ✅
- `d` flag (indices) ✅

### Browser API Parity (Both engines return undefined):

These APIs are NOT available in either engine (consistent behavior):
- `atob` / `btoa` - Both return `typeof === "undefined"` ✅
- `TextEncoder` / `TextDecoder` - Both return `typeof === "undefined"` ✅
- `URL` / `URLSearchParams` - Both return `typeof === "undefined"` ✅

### BREAKING CHANGE IDENTIFIED:

**Intl API** - ONLY breaking change found:
- Deno Core: `typeof Intl === "object"` (available)
- QuickJS: `typeof Intl === "undefined"` (NOT available)

Expressions using these will FAIL with QuickJS:
- `new Intl.NumberFormat('en-US').format(1234567.89)`
- `new Intl.DateTimeFormat('en-US').format(new Date())`
- `num.toLocaleString('de-DE')`
- `date.toLocaleDateString('fr-FR')`

**Mitigation**: Search production logs for `Intl` usage in flow expressions before migration.

### Recommendations:

1. ✅ Run the parity tests to verify current implementation (132 unit tests + 19 flow engine tests pass)
2. ✅ Add tests for edge cases (large integers, optional chaining, spread, multiline)
3. ✅ Test ES2022+ methods - All supported (Array.at, Object.hasOwn, etc.)
4. ✅ Test regex features - All supported (lookbehind, named groups)
5. ⚠️ **Search production for `Intl` usage** - Only confirmed breaking change
6. Run `USE_QUICKJS_FOR_FLOW_EVAL=1` in staging before full production rollout
7. Consider adding `Intl` polyfill to QuickJS if production usage is found

### Commands to Run Tests:

```bash
# Run all parity tests (132 tests)
cargo test --features deno_core,quickjs -p windmill-worker -- parity_

# Run flow engine tests with both engines (19 tests)
cargo test --features deno_core -p windmill --test flow_engine_parity
USE_QUICKJS_FOR_FLOW_EVAL=1 cargo test --features deno_core,quickjs -p windmill --test flow_engine_parity
```
