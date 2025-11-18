# Validation Standards

## Core Principles

- **Validate on Server Side**: Always validate on the server; never trust client-side validation alone for security or data integrity
- **Client-Side for UX**: Use client-side validation to provide immediate user feedback, but duplicate checks server-side
- **Fail Early**: Validate input as early as possible and reject invalid data before processing
- **Specific Error Messages**: Provide clear, field-specific error messages that help users correct their input
- **Allowlists Over Blocklists**: When possible, define what is allowed rather than trying to block everything that's not
- **Type and Format Validation**: Check data types, formats, ranges, and required fields systematically
- **Sanitize Input**: Sanitize user input to prevent injection attacks (SQL, XSS, command injection)
- **Business Rule Validation**: Validate business rules (e.g., sufficient balance, valid dates) at the appropriate application layer
- **Consistent Validation**: Apply validation consistently across all entry points (web forms, API endpoints, background jobs)

## Common Validation Patterns

### Input Validation

1. **Required Fields**: Check that required fields are present and not empty
2. **Type Checking**: Ensure values match expected types (string, number, boolean, etc.)
3. **Format Validation**: Validate formats like email, phone, URL, date, etc.
4. **Range Validation**: Check numeric ranges, string lengths, array sizes
5. **Pattern Matching**: Use regex for specific patterns (postal codes, IDs, etc.)

### Business Logic Validation

1. **State Validation**: Ensure operations are valid for current state (e.g., can't ship unconfirmed order)
2. **Relationship Validation**: Verify relationships between entities are valid
3. **Authorization Validation**: Check user has permission to perform operation
4. **Consistency Validation**: Ensure data consistency across related fields
5. **Temporal Validation**: Validate dates, times, and sequences are logical

### Security Considerations

1. **Injection Prevention**: Sanitize inputs to prevent SQL injection, XSS, command injection
2. **Path Traversal**: Validate file paths to prevent directory traversal attacks
3. **Size Limits**: Enforce size limits on inputs to prevent DoS attacks
4. **Rate Limiting**: Implement rate limiting for validation-heavy operations
5. **Error Message Safety**: Don't expose sensitive information in validation errors
