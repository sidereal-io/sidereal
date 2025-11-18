---
name: testing-standards
description: Use when writing or reviewing tests to ensure adherence to testing standards - focuses on minimal testing during development, deferring edge cases, testing behavior over implementation, clear naming, proper mocking, and fast execution
---

# Testing Standards

## Overview

Apply these testing standards when writing or reviewing tests. The principles prioritize strategic testing over comprehensive coverage during development, focusing on behavior validation rather than implementation details.

## Core Testing Principles

### 1. Write Minimal Tests During Development

**Standard:** Do NOT write tests for every change or intermediate step. Focus on completing the feature implementation first, then add strategic tests only at logical completion points.

**Rationale:**
- Tests validate complete behavior, not every micro-step
- Over-testing during development creates maintenance burden
- Strategic testing is more valuable than exhaustive testing

**Application:**
- Complete feature implementation before adding tests
- Identify logical completion points (e.g., API endpoint complete, component renders correctly)
- Add tests that validate end-to-end behavior at these points
- Avoid writing tests for intermediate refactoring steps

**Example workflow:**
```
1. Implement user authentication feature
2. Verify it works manually
3. Add test: "authenticates user with valid credentials"
4. Add test: "rejects invalid credentials"
5. Move to next feature
```

### 2. Defer Edge Case Testing

**Standard:** Do NOT test edge cases, error states, or validation logic unless they are business-critical. These can be addressed in dedicated testing phases, not during feature development.

**Rationale:**
- Edge cases distract from core functionality
- Business-critical paths deserve focus first
- Edge case testing can be batched in dedicated testing phases

**Application:**
- Focus on happy path during initial development
- Identify business-critical edge cases only (e.g., payment failures, data loss scenarios)
- Document non-critical edge cases for future testing phases
- Add edge case tests only when explicitly required

**Business-critical vs. deferrable:**
```
Business-critical (test now):
- Payment processing failures
- Data corruption scenarios
- Security boundary violations

Deferrable (test later):
- Empty string validation
- Null/undefined edge cases
- Boundary conditions on non-critical inputs
```

### 3. Test Behavior, Not Implementation

**Standard:** Focus tests on what the code does (observable behavior), not how it does it (implementation details), to reduce brittleness and improve maintainability.

**Rationale:**
- Implementation changes frequently; behavior changes rarely
- Tests coupled to implementation break during refactoring
- Behavior tests document actual requirements

**Application:**
- Test observable outputs, not internal state
- Avoid testing private methods or internal variables
- Test through public APIs and user-facing interfaces
- Refactor should not break tests if behavior unchanged

**Examples:**

```typescript
// ❌ BAD: Testing implementation details
test('sets loading state to true', () => {
  component.fetchData();
  expect(component.state.isLoading).toBe(true);
});

// ✅ GOOD: Testing observable behavior
test('displays loading indicator while fetching data', () => {
  component.fetchData();
  expect(screen.getByRole('progressbar')).toBeInTheDocument();
});
```

```typescript
// ❌ BAD: Testing internal method calls
test('calls validateEmail when submitting', () => {
  const validateSpy = jest.spyOn(form, 'validateEmail');
  form.submit();
  expect(validateSpy).toHaveBeenCalled();
});

// ✅ GOOD: Testing validation outcome
test('shows error when email is invalid', () => {
  fillForm({ email: 'invalid' });
  submit();
  expect(screen.getByText('Invalid email')).toBeInTheDocument();
});
```

### 4. Use Clear Test Names

**Standard:** Use descriptive names that explain what's being tested and the expected outcome. Test names should read as specifications.

**Rationale:**
- Clear names document expected behavior
- Failed tests immediately communicate what broke
- Good names reduce need for comments

**Application:**
- Include: what is being tested, under what conditions, expected result
- Use complete sentences or descriptive phrases
- Avoid generic names like "test1" or "user test"
- Test name should explain the requirement

**Pattern:**
```
[Action/Feature] [Condition] → [Expected Result]
```

**Examples:**

```typescript
// ❌ BAD: Vague names
test('user test', () => { ... });
test('validation', () => { ... });
test('works correctly', () => { ... });

// ✅ GOOD: Clear, descriptive names
test('creates user account when valid email and password provided', () => { ... });
test('rejects registration when email already exists', () => { ... });
test('displays validation error when password is too short', () => { ... });
```

**Alternative patterns:**
```typescript
// When/Then pattern
test('when login fails, then displays error message', () => { ... });

// Should pattern
test('should redirect to dashboard after successful login', () => { ... });

// Describe/It pattern (for nested contexts)
describe('User authentication', () => {
  it('authenticates user with valid credentials', () => { ... });
  it('rejects authentication with invalid password', () => { ... });
});
```

### 5. Mock External Dependencies

**Standard:** Isolate units by mocking databases, APIs, file systems, and other external services. Mock at the appropriate level to preserve behavior while eliminating external dependencies.

**Rationale:**
- External dependencies make tests slow and flaky
- Mocking enables fast, reliable, isolated testing
- Proper mocking preserves behavior testing

**Application:**
- Identify external dependencies: databases, HTTP APIs, file systems, third-party services
- Mock at the boundary where external calls occur
- Preserve behavior needed by tests; only mock external operations
- Ensure mocks match real API contracts

**What to mock:**
- Database queries and connections
- HTTP requests to external APIs
- File system operations
- Time/date functions
- Random number generators
- Third-party service SDKs

**Examples:**

```typescript
// ✅ GOOD: Mock database calls
test('retrieves user by ID', async () => {
  const mockDb = {
    query: jest.fn().mockResolvedValue({ id: '123', name: 'Alice' })
  };

  const user = await userService.getById('123', mockDb);
  expect(user.name).toBe('Alice');
});

// ✅ GOOD: Mock HTTP client
test('fetches weather data from API', async () => {
  const mockHttp = {
    get: jest.fn().mockResolvedValue({ temp: 72, condition: 'sunny' })
  };

  const weather = await weatherService.getCurrent('NYC', mockHttp);
  expect(weather.temp).toBe(72);
});
```

**Critical:** Mock at the right level (see testing-antipatterns skill for anti-patterns to avoid).

### 6. Keep Tests Fast

**Standard:** Keep unit tests fast (milliseconds per test) so developers run them frequently during development. Slow tests break development flow.

**Rationale:**
- Fast tests enable rapid feedback loops
- Slow tests get skipped or run infrequently
- Fast execution encourages test-driven development

**Application:**
- Aim for <10ms per unit test
- Mock all I/O operations (network, disk, database)
- Avoid complex setup/teardown
- Use in-memory implementations where possible
- Separate slow integration tests from fast unit tests

**Optimization strategies:**

```typescript
// ❌ SLOW: Real database operations
test('creates user', async () => {
  await db.connect();
  await db.migrate();
  const user = await createUser({ email: 'test@example.com' });
  expect(user.email).toBe('test@example.com');
  await db.cleanup();
});

// ✅ FAST: Mocked database
test('creates user', async () => {
  const mockDb = { insert: jest.fn().mockResolvedValue({ id: '123' }) };
  const user = await createUser({ email: 'test@example.com' }, mockDb);
  expect(user.email).toBe('test@example.com');
});
```

**Test categorization:**
- **Unit tests:** Fast (<10ms), mocked dependencies, run on every save
- **Integration tests:** Slower (seconds), real dependencies, run before commits
- **E2E tests:** Slowest (minutes), full system, run in CI/CD

## Quick Reference

| Principle | Key Guideline |
|-----------|---------------|
| Minimal Testing | Test at logical completion points, not every step |
| Edge Cases | Defer unless business-critical |
| Behavior vs Implementation | Test observable outputs, not internal state |
| Clear Names | Descriptive names that explain requirement |
| Mocking | Mock external dependencies at appropriate level |
| Speed | Unit tests should run in milliseconds |

## When to Apply These Standards

**Use these standards when:**
- Writing new tests for features
- Reviewing test code in pull requests
- Refactoring existing test suites
- Deciding whether to add more test coverage
- Evaluating test quality and maintainability

**Red flags indicating standards violation:**
- Writing tests for every small refactoring step
- Testing implementation details (internal state, private methods)
- Slow unit tests (>100ms per test)
- Vague test names ("test1", "works correctly")
- Tests break during refactoring despite unchanged behavior
- Excessive edge case testing during feature development
