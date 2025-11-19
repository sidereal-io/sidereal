---
name: validation
description: Guide for implementing robust validation in applications. Use when adding validation to forms, APIs, or business logic; reviewing validation code; or ensuring data integrity and security through proper input validation and business rule checking.
---

# Validation

## Overview

Apply these validation standards when implementing input validation, business logic validation, or reviewing validation code. The principles prioritize security, clear error handling, and consistent validation across all entry points.

## Core Validation Principles

### 1. Server-Side Validation is Mandatory

**Standard:** Always validate on the server, regardless of client-side validation. Never trust data from the client.

**Rationale:**
- Client-side validation can be bypassed
- Server is the only trusted boundary
- All entry points (forms, APIs, imports) must validate server-side

**Application:**
- Duplicate all client-side validations on the server
- Validate immediately when data enters the system
- Apply security checks (sanitization, authorization) server-side only

```typescript
// ❌ BAD: Only client-side validation
const form = useForm({
  validate: { email: (v) => isEmail(v) }
});

// ✅ GOOD: Server-side validation
app.post('/users', (req, res) => {
  const { email } = req.body;
  if (!email || !isEmail(email)) {
    return res.status(400).json({
      field: 'email',
      message: 'Email must be a valid email address'
    });
  }
  // proceed...
});
```

### 2. Validate in Layers

**Standard:** Implement validation at multiple layers—client (UX), server (security), and database (integrity)—with each layer serving a distinct purpose.

**Rationale:**
- Defense in depth prevents data corruption
- Each layer catches different classes of issues
- Database constraints are the final safety net

**Application:**

**Client-Side (UX-focused):**
- Immediate feedback on format and required fields
- Validate on blur/change events
- Never for security-critical checks

**Server-Side (Security-focused):**
- All client validations duplicated
- Authentication and authorization
- Business rules and cross-field validation

**Database-Level (Integrity-focused):**
- NOT NULL, UNIQUE, CHECK constraints
- Foreign key relationships
- Type constraints

```typescript
// Client: immediate feedback
<input
  type="email"
  required
  onBlur={(e) => setError(!isEmail(e.target.value))}
/>

// Server: security and business rules
function createUser(data: UserInput) {
  if (!data.email || !isEmail(data.email)) {
    throw new ValidationError('email', 'Invalid email');
  }
  if (await userExists(data.email)) {
    throw new ValidationError('email', 'Email already registered');
  }
}

// Database: constraints
CREATE TABLE users (
  email VARCHAR(255) NOT NULL UNIQUE,
  CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$')
);
```

### 3. Fail Fast with Clear Errors

**Standard:** Validate early and return specific, actionable error messages. Users should understand exactly what to fix.

**Rationale:**
- Early validation prevents wasted processing
- Clear errors reduce support burden
- Field-specific errors improve UX

**Application:**
- Validate before any processing or database operations
- Include field name/path in errors
- Provide actionable guidance, not just "invalid"
- Return multiple errors when possible

```typescript
// ❌ BAD: Generic error
throw new Error('Invalid input');

// ❌ BAD: Technical error
throw new Error('Validation failed: email pattern mismatch');

// ✅ GOOD: Clear, actionable error
return {
  field: 'email',
  code: 'invalid_format',
  message: 'Email must be a valid email address'
};

// ✅ GOOD: Multiple errors at once
return {
  errors: [
    { field: 'email', message: 'Email is required' },
    { field: 'password', message: 'Password must be at least 8 characters' }
  ]
};
```

### 4. Use Allowlists Over Blocklists

**Standard:** Define what is allowed rather than what is forbidden. Allowlists are more secure and maintainable.

**Rationale:**
- Blocklists always miss edge cases
- Attackers find bypasses for blocklists
- Allowlists are explicit and auditable

**Application:**
- Validate against known good values
- Use enums/sets for categorical data
- Regex patterns should match valid formats, not exclude invalid ones

```typescript
// ❌ BAD: Blocklist approach
function validateRole(role: string) {
  const forbidden = ['admin', 'superuser', 'root'];
  if (forbidden.includes(role)) {
    throw new Error('Invalid role');
  }
}

// ✅ GOOD: Allowlist approach
const VALID_ROLES = ['user', 'editor', 'viewer'] as const;
type Role = typeof VALID_ROLES[number];

function validateRole(role: string): role is Role {
  return VALID_ROLES.includes(role as Role);
}
```

### 5. Structure Errors Consistently

**Standard:** Use a consistent error response format across all validation endpoints. Include field, code, and message.

**Rationale:**
- Consistent format simplifies client error handling
- Machine-readable codes enable automation
- Human-readable messages improve UX

**Application:**
- Define a standard error schema for the project
- Include field path for nested objects
- Use HTTP status codes appropriately

```typescript
// Standard error structure
interface ValidationError {
  field: string;      // 'email' or 'address.zipCode'
  code: string;       // 'required', 'invalid_format', 'too_short'
  message: string;    // 'Email is required'
}

// HTTP status codes
// 400 Bad Request - malformed input
// 422 Unprocessable Entity - semantic validation failure
// 401 Unauthorized - authentication failure
// 403 Forbidden - authorization failure

// API response
app.post('/users', (req, res) => {
  const errors = validateUser(req.body);
  if (errors.length > 0) {
    return res.status(422).json({ errors });
  }
});
```

## Input Validation Patterns

Common patterns for validating different types of input:

### Required Fields

```typescript
function validateRequired(value: unknown, field: string): ValidationError | null {
  if (value === undefined || value === null || value === '') {
    return { field, code: 'required', message: `${field} is required` };
  }
  return null;
}
```

### Type and Format Validation

```typescript
// Email format
const EMAIL_REGEX = /^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$/;

function validateEmail(email: string): boolean {
  return EMAIL_REGEX.test(email);
}

// UUID format
const UUID_REGEX = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

// Date validation
function validateDateRange(start: Date, end: Date): ValidationError | null {
  if (end <= start) {
    return {
      field: 'endDate',
      code: 'invalid_range',
      message: 'End date must be after start date'
    };
  }
  return null;
}
```

### Range and Length Validation

```typescript
function validateLength(
  value: string,
  field: string,
  min: number,
  max: number
): ValidationError | null {
  if (value.length < min) {
    return {
      field,
      code: 'too_short',
      message: `${field} must be at least ${min} characters`
    };
  }
  if (value.length > max) {
    return {
      field,
      code: 'too_long',
      message: `${field} must be at most ${max} characters`
    };
  }
  return null;
}

function validateRange(
  value: number,
  field: string,
  min: number,
  max: number
): ValidationError | null {
  if (value < min || value > max) {
    return {
      field,
      code: 'out_of_range',
      message: `${field} must be between ${min} and ${max}`
    };
  }
  return null;
}
```

## Business Logic Validation

### State Validation

Verify operations are valid for the current entity state:

```typescript
function validateOrderCancellation(order: Order): ValidationError | null {
  const nonCancellableStates = ['shipped', 'delivered', 'cancelled'];

  if (nonCancellableStates.includes(order.status)) {
    return {
      field: 'status',
      code: 'invalid_state',
      message: `Cannot cancel order after it has ${order.status}`
    };
  }
  return null;
}

function validateWithdrawal(account: Account, amount: number): ValidationError | null {
  if (amount > account.balance) {
    return {
      field: 'amount',
      code: 'insufficient_funds',
      message: `Cannot withdraw more than available balance (${account.balance})`
    };
  }
  return null;
}
```

### Relationship Validation

Verify referenced entities exist and are accessible:

```typescript
async function validateProductExists(
  productId: string,
  db: Database
): Promise<ValidationError | null> {
  const product = await db.products.findById(productId);
  if (!product) {
    return {
      field: 'productId',
      code: 'not_found',
      message: 'Product not found'
    };
  }
  return null;
}

async function validateOwnership(
  resourceId: string,
  userId: string,
  db: Database
): Promise<ValidationError | null> {
  const resource = await db.resources.findById(resourceId);
  if (resource?.ownerId !== userId) {
    return {
      field: 'resourceId',
      code: 'forbidden',
      message: 'Access denied'
    };
  }
  return null;
}
```

### Cross-Field Validation

Ensure related fields are consistent:

```typescript
function validateOrderTotal(order: Order): ValidationError | null {
  const calculatedTotal = order.items.reduce(
    (sum, item) => sum + item.price * item.quantity,
    0
  );

  if (Math.abs(order.total - calculatedTotal) > 0.01) {
    return {
      field: 'total',
      code: 'mismatch',
      message: 'Total must equal sum of line items'
    };
  }
  return null;
}
```

## Security Validation

### Input Sanitization

Prevent injection attacks through proper validation and parameterization:

```typescript
// ❌ BAD: SQL injection vulnerability
const user = await db.query(`SELECT * FROM users WHERE id = '${userId}'`);

// ✅ GOOD: Parameterized query
const user = await db.query('SELECT * FROM users WHERE id = $1', [userId]);

// ❌ BAD: Command injection
exec(`convert ${filename} output.png`);

// ✅ GOOD: Avoid shell execution with user input, or use allowlist
const ALLOWED_EXTENSIONS = ['.jpg', '.png', '.gif'];
if (!ALLOWED_EXTENSIONS.includes(path.extname(filename))) {
  throw new ValidationError('filename', 'Invalid file type');
}
```

### Path Traversal Prevention

```typescript
function validateFilePath(userPath: string, baseDir: string): ValidationError | null {
  const resolved = path.resolve(baseDir, userPath);

  if (!resolved.startsWith(baseDir)) {
    return {
      field: 'path',
      code: 'invalid_path',
      message: 'Invalid file path'
    };
  }
  return null;
}
```

### Size Limits

```typescript
// Express middleware for request size limits
app.use(express.json({ limit: '100kb' }));

// File upload validation
function validateFileSize(file: File, maxBytes: number): ValidationError | null {
  if (file.size > maxBytes) {
    return {
      field: 'file',
      code: 'too_large',
      message: `File must be smaller than ${maxBytes / 1024 / 1024}MB`
    };
  }
  return null;
}
```

### Secure Error Messages

```typescript
// ❌ BAD: Exposes system information
if (!user) {
  throw new Error('User not found in database');
}
if (!bcrypt.compare(password, user.passwordHash)) {
  throw new Error('Password does not match hash');
}

// ✅ GOOD: Generic message, detailed logging
if (!user || !bcrypt.compare(password, user.passwordHash)) {
  logger.warn('Failed login attempt', { email, ip: req.ip });
  throw new ValidationError('credentials', 'Invalid email or password');
}
```

## Testing Validation

Test validation logic thoroughly but strategically:

```typescript
describe('User validation', () => {
  // Test valid inputs pass
  it('accepts valid user data', () => {
    const result = validateUser({
      email: 'test@example.com',
      password: 'securePass123'
    });
    expect(result.errors).toHaveLength(0);
  });

  // Test invalid inputs fail with correct errors
  it('rejects invalid email format', () => {
    const result = validateUser({
      email: 'not-an-email',
      password: 'securePass123'
    });
    expect(result.errors).toContainEqual({
      field: 'email',
      code: 'invalid_format',
      message: expect.stringContaining('valid email')
    });
  });

  // Test security scenarios
  it('prevents SQL injection in email field', () => {
    const result = validateUser({
      email: "'; DROP TABLE users; --",
      password: 'securePass123'
    });
    expect(result.errors).toContainEqual(
      expect.objectContaining({ field: 'email' })
    );
  });

  // Test boundary conditions
  it('enforces minimum password length', () => {
    const result = validateUser({
      email: 'test@example.com',
      password: 'short'
    });
    expect(result.errors).toContainEqual(
      expect.objectContaining({
        field: 'password',
        code: 'too_short'
      })
    );
  });
});
```

## Quick Reference

| Principle | Key Guideline |
|-----------|---------------|
| Server-Side | Always validate server-side, never trust client |
| Layered Validation | Client (UX), Server (security), Database (integrity) |
| Fail Fast | Validate early, return specific actionable errors |
| Allowlists | Define what is allowed, not what is forbidden |
| Consistent Errors | Standard format: field, code, message |
| Security | Parameterize queries, validate paths, limit sizes |

## When to Apply These Standards

**Use these standards when:**
- Implementing validation for forms, APIs, or data imports
- Adding business logic validation
- Reviewing validation code for security or completeness
- Designing validation strategy for new features

**Red flags indicating standards violation:**
- No server-side validation (client-only)
- Generic error messages ("Invalid input")
- Blocklist-based validation
- Inconsistent error response formats
- Missing size limits on inputs
- Exposed system information in errors
