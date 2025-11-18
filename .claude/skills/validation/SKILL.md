---
name: validation
description: Guide for implementing robust validation in applications. Use this skill when adding validation to forms, APIs, or business logic; reviewing validation code; or ensuring data integrity and security through proper input validation and business rule checking.
---

# Validation

## Overview

This skill provides guidance for implementing comprehensive validation across applications, covering both input validation (forms, APIs) and business logic validation. Apply the principles and patterns from this skill to ensure data integrity, security, and proper error handling throughout the application.

## When to Use This Skill

Invoke this skill when:

- Implementing validation for forms, API endpoints, or data inputs
- Adding business logic validation for domain rules and constraints
- Reviewing validation code for security vulnerabilities or completeness
- Designing validation strategy for a new feature or service
- Debugging validation issues or improving error messages
- Ensuring consistent validation across multiple entry points

## Core Validation Principles

Before implementing validation, review the detailed standards in `references/standards.md`. The core principles include:

1. **Server-side validation is mandatory** - Never trust client-side validation alone
2. **Client-side validation improves UX** - Provide immediate feedback, but always duplicate server-side
3. **Fail early** - Validate as soon as data enters the system
4. **Clear error messages** - Provide specific, actionable feedback
5. **Allowlists over blocklists** - Define what is allowed rather than what is forbidden
6. **Consistent validation** - Apply validation uniformly across all entry points

## Implementation Approach

### 1. Identify Validation Requirements

Before implementing validation, determine:

- **What data needs validation?** (user inputs, API payloads, file uploads, etc.)
- **What are the validation rules?** (required fields, formats, ranges, business constraints)
- **Where does validation occur?** (client-side, server-side, database constraints)
- **What error messages are appropriate?** (specific enough to help, vague enough for security)

### 2. Layer Validation Appropriately

Implement validation in layers:

**Client-Side (Optional, UX-focused)**
- Immediate feedback for user experience
- Format validation (email, phone, date patterns)
- Required field checks
- Field-level validation on blur/change events

**Server-Side (Required, Security-focused)**
- All client-side validations duplicated
- Authentication and authorization checks
- Business rule validation
- Data sanitization and security checks
- Cross-field validation
- Database constraint validation

**Database-Level (Defense in depth)**
- NOT NULL constraints for required fields
- UNIQUE constraints for uniqueness requirements
- CHECK constraints for simple business rules
- Foreign key constraints for referential integrity

### 3. Input Validation Patterns

Follow these patterns for input validation:

**Required Fields**
```
Check that fields are present and not empty/null/undefined
Validate before processing any data
Return field-specific error: "Email is required"
```

**Type and Format Validation**
```
Verify data types match expectations (string, number, boolean, etc.)
Validate formats: email, phone, URL, date, UUID, etc.
Use allowlist patterns where possible
Return specific error: "Email must be a valid email address"
```

**Range and Length Validation**
```
Check numeric ranges (min/max values)
Validate string lengths (min/max characters)
Verify array/collection sizes
Return specific error: "Password must be at least 8 characters"
```

**Pattern Validation**
```
Use regex for specific patterns (postal codes, IDs, codes)
Validate against allowlists of known valid values
Check enum/option values against allowed set
Return specific error: "Postal code must match format A1A 1A1"
```

### 4. Business Logic Validation

Implement business rule validation:

**State Validation**
```
Verify operations are valid for current entity state
Example: Can't cancel a shipped order, can't withdraw more than balance
Check state transitions are allowed
Return business-specific error: "Cannot cancel order after it has shipped"
```

**Relationship Validation**
```
Verify referenced entities exist and are accessible
Check relationships are valid (user owns resource, etc.)
Validate foreign key relationships
Return specific error: "Product not found" or "Access denied"
```

**Temporal Validation**
```
Validate dates and times are logical (end after start, future dates, etc.)
Check business hours, availability, scheduling conflicts
Verify time-based constraints
Return specific error: "End date must be after start date"
```

**Consistency Validation**
```
Ensure related fields are consistent with each other
Validate calculated fields match inputs
Check aggregate constraints (total matches sum of parts)
Return specific error: "Total amount must equal sum of line items"
```

### 5. Security Considerations

Always implement security-focused validation:

**Input Sanitization**
```
Sanitize all user inputs before processing or storage
Prevent SQL injection: use parameterized queries, ORM protection
Prevent XSS: escape HTML, use Content Security Policy
Prevent command injection: avoid shell execution with user input
Validate and sanitize file uploads
```

**Path Traversal Prevention**
```
Validate file paths don't contain "../" or absolute paths
Restrict file operations to specific directories
Use allowlists for file extensions
```

**Size Limits**
```
Enforce maximum sizes for strings, arrays, files
Prevent denial of service through oversized inputs
Set reasonable limits based on business needs
```

**Error Message Safety**
```
Don't expose sensitive information in errors
Generic errors for authentication: "Invalid credentials" not "User not found"
Don't reveal system internals or database structure
Log detailed errors server-side, show safe messages to users
```

### 6. Error Handling and Reporting

Structure validation errors consistently:

**Error Structure**
```
Use consistent error response format
Include field name/path for field-specific errors
Provide error code and human-readable message
Support multiple errors per request
Example: { field: "email", code: "invalid_format", message: "Email must be a valid email address" }
```

**HTTP Status Codes**
```
400 Bad Request - for validation failures
401 Unauthorized - for authentication failures
403 Forbidden - for authorization failures
422 Unprocessable Entity - for semantic validation failures
```

**User-Facing Messages**
```
Clear and specific enough to help user fix the issue
Professional tone, not blaming user
Actionable guidance where possible
Example: "Email must be a valid email address" not "Invalid input"
```

## Common Validation Libraries and Tools

Depending on the technology stack, use appropriate validation libraries:

**JavaScript/TypeScript**
- Zod - Type-safe schema validation
- Yup - Object schema validation
- Joi - Schema validation
- class-validator - Decorator-based validation
- validator.js - String validators

**Python**
- Pydantic - Data validation using type hints
- marshmallow - Object serialization and validation
- cerberus - Lightweight validation
- jsonschema - JSON Schema validation

**General**
- JSON Schema - Language-agnostic schema validation
- OpenAPI/Swagger - API contract validation

## Testing Validation

When writing tests for validation:

- **Focus on boundary conditions**: Test edge cases for ranges, lengths, formats
- **Test both valid and invalid inputs**: Ensure valid data passes and invalid data fails
- **Test error messages**: Verify error messages are correct and helpful
- **Test security scenarios**: Include injection attempts, path traversal, oversized inputs
- **Test business rules**: Verify business logic validation catches invalid states

Refer to `references/standards.md` for the complete validation standards and additional patterns.
