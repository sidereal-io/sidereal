---
name: code-style
description: Guide for writing clean, maintainable TypeScript/JavaScript code following best practices. Use when writing new code or refactoring existing code to ensure consistent style, readability, and maintainability.
---

# Code Style

## Overview

This skill provides guidance for writing clean, maintainable TypeScript/JavaScript code. Apply these principles when writing new code or refactoring existing code to ensure consistency, readability, and long-term maintainability.

## Core Principles

### Consistent Naming Conventions

Establish and follow naming conventions for variables, functions, classes, and files across the codebase:

- **Variables and functions**: Use camelCase (e.g., `getUserData`, `isValid`)
- **Classes and types**: Use PascalCase (e.g., `UserService`, `ApiResponse`)
- **Constants**: Use UPPER_SNAKE_CASE for true constants (e.g., `MAX_RETRY_COUNT`)
- **Files**: Match the primary export (e.g., `UserService.ts` for a `UserService` class)
- **Private members**: Prefix with underscore or use TypeScript private modifier

### Meaningful Names

Choose descriptive names that reveal intent. Avoid abbreviations and single-letter variables except in narrow contexts:

**Good:**
```typescript
const activeUserCount = users.filter(user => user.isActive).length;
function calculateTotalPrice(items: CartItem[]): number { ... }
```

**Avoid:**
```typescript
const cnt = users.filter(u => u.isActive).length;
function calc(arr: any[]): number { ... }
```

**Acceptable exceptions:**
- Loop indices: `i`, `j`, `k` in simple loops
- Common abbreviations in narrow scope: `err` for error, `ctx` for context
- Well-established domain terms: `id`, `url`, `api`

### Small, Focused Functions

Keep functions small and focused on a single task for better readability and testability:

- Each function should do one thing well
- Aim for functions that fit on a single screen (typically < 20-30 lines)
- Extract complex logic into helper functions with descriptive names
- If a function needs extensive comments to explain what it does, it's probably too complex

**Example of refactoring:**
```typescript
// Before: Large, unfocused function
function processOrder(order: Order) {
  // validation logic (10 lines)
  // inventory check logic (15 lines)
  // payment processing logic (20 lines)
  // notification logic (10 lines)
}

// After: Small, focused functions
function processOrder(order: Order) {
  validateOrder(order);
  checkInventory(order);
  processPayment(order);
  sendOrderConfirmation(order);
}
```

### Automated Formatting

Maintain consistent code style through automated tools:

- Configure Prettier or similar formatter for the project
- Set up pre-commit hooks or CI checks to enforce formatting
- Configure editor to format on save
- Consistent indentation (spaces vs tabs), line breaks, and spacing

### Remove Dead Code

Delete unused code, commented-out blocks, and imports rather than leaving them as clutter:

- Remove unused functions, variables, and imports
- Delete commented-out code blocks (version control preserves history)
- Clean up experimental code that didn't make it to production
- Remove obsolete comments that no longer apply

**Avoid:**
```typescript
import { oldFunction } from './legacy'; // unused import

function processData(data: any) {
  // const result = oldFunction(data); // old approach
  const result = newFunction(data);
  // console.log('Debug:', result); // debugging code
  return result;
}
```

### DRY Principle (Don't Repeat Yourself)

Avoid duplication by extracting common logic into reusable functions or modules:

- If the same code appears twice, extract it into a function
- If the same logic appears across files, create a shared module
- Use higher-order functions to abstract common patterns
- Balance DRY with readability (don't over-abstract)

### Self-Documenting Code

Write code that explains itself through clear structure and naming:

- Use descriptive variable and function names instead of comments
- Structure code to reveal intent (e.g., early returns for validation)
- Prefer explicit over clever (readability > brevity)
- Use TypeScript types to document expected shapes and contracts

**Example:**
```typescript
// Self-documenting through naming and structure
function canUserAccessResource(user: User, resource: Resource): boolean {
  if (!user.isAuthenticated) return false;
  if (user.isAdmin) return true;
  return resource.ownerId === user.id;
}
```

### Minimal, Helpful Comments

Add concise, minimal comments to explain large sections of code logic:

- Explain **why**, not **what** (the code already shows what)
- Document non-obvious business logic or algorithms
- Add context for complex workarounds or edge cases
- Use JSDoc/TSDoc for public APIs and exported functions

**Good comments:**
```typescript
// Retry with exponential backoff to handle rate limiting
await retryWithBackoff(apiCall);

/**
 * Calculates the adjusted price after applying tiered discounts.
 * Discount tiers: 0-10 items (0%), 11-50 (10%), 51+ (20%)
 */
function calculatePrice(items: Item[]): number { ... }
```

**Avoid:**
```typescript
// Set x to 5
const x = 5;

// Loop through users
for (const user of users) { ... }
```

### Don't Comment Changes or Fixes

Do not leave code comments that explain changes, fixes, or who made them:

- Version control (git) tracks changes and authorship
- Comments describing changes become stale and misleading
- Focus comments on explaining the current state, not the history

**Avoid:**
```typescript
// Fixed bug where null values caused crash - John 2024-01-15
if (value !== null) { ... }

// TODO: Refactor this later
// Changed from oldMethod to newMethod because of performance
```

### Backward Compatibility Only When Required

Unless specifically instructed otherwise, assume backward compatibility is not required:

- Don't write extra code to maintain deprecated APIs
- Clean up old interfaces and implementations
- Remove compatibility layers for legacy systems
- Focus on the current requirements, not hypothetical future needs

**Avoid unnecessary compatibility code:**
```typescript
// Unnecessary unless explicitly required
function getData(id: string | number) { // supporting both for "compatibility"
  const normalizedId = typeof id === 'number' ? String(id) : id;
  // ...
}

// Simpler if backward compatibility isn't needed
function getData(id: string) {
  // ...
}
```

### Consistent Indentation

Use consistent indentation throughout the codebase:

- Configure editor/linter to enforce indentation rules
- Follow project conventions (typically 2 spaces for TS/JS)
- Ensure proper indentation in nested structures
- Use automated formatting to maintain consistency

## When to Apply This Skill

Apply these principles proactively in the following scenarios:

1. **Writing new code**: When creating new functions, classes, or modules
2. **Refactoring existing code**: When cleaning up or restructuring code
3. **Code reviews**: When reviewing code for style and maintainability issues

Do not wait for explicit requests to apply these standards. They should be the default approach to writing code.

## Balance and Judgment

While these principles are important, apply them with judgment:

- **Pragmatism over perfection**: Don't let perfect be the enemy of good
- **Context matters**: Small scripts may not need the same rigor as production APIs
- **Team conventions**: Follow established team/project patterns when they exist
- **Readability first**: If a principle makes code harder to read, reconsider
