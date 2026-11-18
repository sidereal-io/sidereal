## REMOVED Requirements

### Requirement: Caller-supplied values cannot forge a line

**Reason**: Replaced by "Caller-supplied values are escaped in a line". A claimed username can no longer hold a control character, so the scenario "A line feed in a claimed username" cannot happen: that claim now gets a 400 and writes no line. The escaping rule itself is unchanged.

**Migration**: None. Lines are written exactly as before.

## ADDED Requirements

### Requirement: Caller-supplied values are escaped in a line

Each `username` and `user_agent` value SHALL be written in double quotes. Inside the quotes, a backslash SHALL be written as `\\` and a double quote as `\"`. Every control character, such as a line feed, SHALL be written as `\u{<hex>}`, with its code point in lowercase hexadecimal. A line SHALL therefore hold no line break and no unescaped double quote inside a value.

#### Scenario: A tab in the user agent
- **WHEN** a caller fails sign-in with a `User-Agent` header of `Firefox`, a tab, and `140`
- **THEN** the new line holds `user_agent="Firefox\u{9}140"`

#### Scenario: A quote and a backslash in a claimed username
- **WHEN** no administrator exists and a caller claims with username `sa"m\` and a valid password
- **THEN** the line holds `username="sa\"m\\"`
