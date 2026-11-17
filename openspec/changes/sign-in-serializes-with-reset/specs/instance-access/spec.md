## MODIFIED Requirements

### Requirement: Sign-in creates a session

`POST /api/session` takes the JSON body `{"username":"<u>","password":"<p>"}`. When the username exists and the password matches its hash, it SHALL create a session row and answer 200 with the body `{"username":"<u>","role":"<role>"}` and a `Set-Cookie` header for `sidereal_session` whose value is the token. The session row SHALL hold the SHA-256 hash of a new random 32-byte token, the authentication type `password`, and its created, last-used and expiry times. When the username does not exist or the password does not match, it SHALL answer 401 with the body `{"error":"invalid credentials"}` and create nothing. It SHALL verify a password hash whether or not the username exists, so the two failures take the same work. A body that is not a JSON object with string `username` and `password` SHALL answer 400 with the same body. Each of these answers applies only while the request's client address is not limited; a limited address is answered 429, as the sign-in limit requirements say.

A sign-in SHALL create its session only while the user's password hash is still the one its password matched. When an access recovery replaces that hash before the session row is saved, the sign-in SHALL answer as a wrong password does: 401 with the body `{"error":"invalid credentials"}`, no cookie, and no session row. A sign-in whose session row is saved first SHALL answer 200, and the access recovery ends that session as the reset state requires. So no session created with a replaced password outlives the access recovery that replaced it.

#### Scenario: Correct credentials
- **WHEN** Sam signs in with the username and password of the claim
- **THEN** the status is 200, the body is `{"username":"sam","role":"admin"}`, the response sets `sidereal_session`, and the `sessions` table holds one row whose token hash is the SHA-256 of the cookie value, with authentication type `password`

#### Scenario: Wrong password
- **WHEN** a caller signs in with username `sam` and a wrong password
- **THEN** the status is 401, the body is `{"error":"invalid credentials"}`, no cookie is set, and the `sessions` table is unchanged

#### Scenario: Unknown username
- **WHEN** a caller signs in with username `nobody` and any password
- **THEN** the status, body and headers are identical to the wrong-password answer

#### Scenario: Malformed body
- **WHEN** a caller sends `POST /api/session` with a body that is not a JSON object with string `username` and `password`
- **THEN** the status is 400 and the body is `{"error":"invalid credentials"}`

#### Scenario: Sign-in with the old password during an access recovery
- **WHEN** an access recovery has replaced Sam's password hash but not yet committed, a caller signs in as `sam` with the old password, and the access recovery then commits
- **THEN** the sign-in answers 401 with the body `{"error":"invalid credentials"}`, no cookie is set, and the `sessions` table is empty
