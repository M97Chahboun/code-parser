# Worked Example: Navigating a Flutter Project

This example shows the full two-phase workflow on a real Flutter project structure.

## Setup

Project: a Flutter e-commerce app with ~8,000 lines across 24 files.

**Task from user:** "Add input validation to the checkout form — the email field should validate format and the card number field should only accept digits."

---

## Phase 1: Index the project

```bash
code-parser ./lib --format pretty
```

Output (abridged):

```json
[
  { "file": "lib/screens/checkout_screen.dart", "language": "dart",
    "classes": [
      { "name": "CheckoutScreen", "kind": "class", "line_start": 8, "line_end": 142,
        "methods": [
          { "name": "CheckoutScreen", "line_start": 12, "line_end": 12 },
          { "name": "createState",    "line_start": 15, "line_end": 15 }
        ]
      },
      { "name": "_CheckoutScreenState", "kind": "class", "line_start": 18, "line_end": 142,
        "methods": [
          { "name": "initState",      "line_start": 22, "line_end": 28 },
          { "name": "_validateEmail", "line_start": 30, "line_end": 35 },
          { "name": "_submitForm",    "line_start": 37, "line_end": 68 },
          { "name": "build",          "line_start": 70, "line_end": 141 }
        ]
      }
    ]
  },
  { "file": "lib/validators/form_validators.dart", "language": "dart",
    "classes": [
      { "name": "FormValidators", "kind": "class", "line_start": 3, "line_end": 44,
        "methods": [
          { "name": "email",       "line_start": 5,  "line_end": 12 },
          { "name": "phone",       "line_start": 14, "line_end": 21 },
          { "name": "required",    "line_start": 23, "line_end": 28 }
        ]
      }
    ]
  },
  { "file": "lib/models/order.dart", "language": "dart", "classes": [ ... ] },
  { "file": "lib/services/payment_service.dart", "language": "dart", "classes": [ ... ] }
  // ... 20 more files, none relevant to this task
]
```

**Token cost so far: ~800 tokens** (index of 24 files)

---

## Phase 2: Identify relevant code

From the index, the task touches:
1. `_CheckoutScreenState._validateEmail` (lines 30–35) — already exists, might need updating
2. `_CheckoutScreenState.build` (lines 70–141) — where the form fields live
3. `FormValidators` (lines 3–44) — utility class, may need a `cardNumber` method

That's 3 reads. Total: ~200 lines instead of 8,000.

---

## Phase 3: Targeted reads

**Read 1 — existing email validator:**
```bash
sed -n '30,35p' lib/screens/checkout_screen.dart
```
```dart
  String? _validateEmail(String? value) {
    if (value == null || value.isEmpty) return 'Required';
    if (!value.contains('@')) return 'Invalid email';
    return null;
  }
```
Too simple — needs a proper regex. ✓ noted.

**Read 2 — the build method (form fields only):**
```bash
sed -n '70,141p' lib/screens/checkout_screen.dart
```
Found: `TextFormField` for email at line 88, card number field at line 104.
Both use `validator:` parameter. ✓ noted the field names and positions.

**Read 3 — FormValidators:**
```bash
sed -n '3,44p' lib/validators/form_validators.dart
```
```dart
class FormValidators {
  static String? email(String? value) {
    if (value == null || value.isEmpty) return 'Email is required';
    final regex = RegExp(r'^[\w.-]+@[\w.-]+\.[a-z]{2,}$');
    return regex.hasMatch(value) ? null : 'Enter a valid email';
  }

  static String? phone(String? value) { ... }
  static String? required(String? value) { ... }
  // no cardNumber method yet
}
```

**Token cost for all three reads: ~480 tokens**

---

## Phase 4: Act

**Total tokens used:** 800 (index) + 480 (reads) = **1,280 tokens**

Compare to naive approach: read all 24 files = **~60,000 tokens** — a **98% reduction**.

### Changes to make

**1. Add `cardNumber` to `FormValidators` (after line 28):**
```dart
  static String? cardNumber(String? value) {
    if (value == null || value.isEmpty) return 'Card number is required';
    if (!RegExp(r'^\d{16}$').hasMatch(value.replaceAll(' ', ''))) {
      return 'Enter a valid 16-digit card number';
    }
    return null;
  }
```

**2. Update email validator in `_CheckoutScreenState` (lines 30–35):**
```dart
  String? _validateEmail(String? value) =>
      FormValidators.email(value);  // delegate to the utility class
```

**3. Update card field validator in build (around line 104):**
```dart
validator: FormValidators.cardNumber,
```

All edits are surgical. No full files needed. Done. ✓

---

## Key takeaways from this example

1. **Index first always** — even on a 24-file project the index fit in ~800 tokens
2. **The index revealed `FormValidators` existed** — without it, the model might have duplicated logic
3. **Only 3 line-range reads were needed** out of 24 files and ~200 methods
4. **98% token reduction** without losing any relevant context
5. **The model never loaded `order.dart`, `payment_service.dart`, or 20 other files** — they weren't needed
