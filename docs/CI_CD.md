# CI/CD Pipeline Documentation

This document describes the continuous integration and continuous deployment (CI/CD) pipeline for Sidereal, including automated builds, testing, security scanning, and release processes.

## Overview

Sidereal uses GitHub Actions for CI/CD automation with three main workflows:

1. **Build and Push** - Automated builds and container registry pushes
2. **PR Testing** - Pull request validation and testing
3. **Release** - Semantic versioning releases with artifacts

## Workflows

### 1. Build and Push Workflow (`docker-build-push.yml`)

**Triggers:**
- Push to `main` branch
- Pull requests to `main` branch
- Manual workflow dispatch

**Purpose:** Build snapshot Docker images with full validation and security scanning.

**Steps:**
1. **Code Validation**
   - TypeScript type checking (`npm run check`)
   - Application build verification (`npm run build`)

2. **Docker Build**
   - Multi-architecture builds (linux/amd64, linux/arm64)
   - Images pushed only on `main` branch (not PRs)
   - Automatic tagging:
     - `latest` - Latest main branch build
     - `main-<SHA>` - Git commit SHA
     - `YYYYMMDD-HHmmss` - Timestamp

3. **Security Scanning**
   - Trivy vulnerability scanner
   - Fails build on CRITICAL vulnerabilities
   - Results uploaded to GitHub Security tab

4. **Artifacts Generation**
   - SBOM (Software Bill of Materials) in SPDX format
   - Application snapshot (built dist files)
   - Uploaded as build artifacts with 30-day retention

### 2. PR Test Workflow (`docker-build-test.yml`)

**Triggers:**
- Pull requests that modify:
  - Dockerfile
  - docker-compose.yml
  - Package files
  - Source code (apps/, packages/, tools/)

**Purpose:** Validate PRs before merging.

**Steps:**
1. **Code Quality**
   - TypeScript compilation check
   - Application build test
   - Dockerfile linting with Hadolint

2. **Docker Build Test**
   - Build without pushing
   - Image structure validation
   - Container smoke test

3. **Security Analysis**
   - Vulnerability scanning
   - Report added as PR comment
   - Non-blocking (informational only)

4. **Build Artifacts**
   - Docker image snapshot (7-day retention)
   - Application snapshot (7-day retention)
   - Available for download and testing

5. **PR Feedback**
   - Automatic comment with test results
   - Vulnerability report in collapsible section
   - Artifact download links
   - Links to full Action logs

### 3. Release Workflow (`release.yml`)

**Triggers:**
- Annotated tags matching `v*.*.*` pattern (e.g., v1.0.0)

**Purpose:** Create production releases with versioned artifacts.

**Steps:**
1. **Tag Validation**
   - Ensures semantic versioning format
   - Example: `v1.2.3`

2. **Release Artifacts**
   - Distribution archive: `sidereal-v1.0.0-dist.tar.gz`
   - Source archive: `sidereal-v1.0.0-source.tar.gz`
   - SBOM: `sbom-v1.0.0.spdx.json`

3. **Docker Images**
   - Tagged with version: `ghcr.io/user/repo:v1.0.0`
   - Also tagged: `latest`, `1`, `1.0`

4. **GitHub Release**
   - Automatic changelog generation
   - Artifact uploads
   - Release notes

## Container Registry

Images are published to GitHub Container Registry (ghcr.io):

```bash
# Pull latest version
docker pull ghcr.io/<owner>/<repo>:latest

# Pull specific version
docker pull ghcr.io/<owner>/<repo>:v1.0.0

# Pull snapshot build
docker pull ghcr.io/<owner>/<repo>:main-abc123
```

## Security

### Vulnerability Scanning

All Docker images are scanned with Trivy for known vulnerabilities:

- **Build Workflow**: Fails on CRITICAL vulnerabilities
- **PR Workflow**: Informational only (doesn't block PRs)
- **Release Workflow**: Fails on CRITICAL vulnerabilities

Results are:
- Uploaded to GitHub Security tab
- Available in workflow summaries
- Included in PR comments

### SBOM (Software Bill of Materials)

Every build generates an SBOM containing:
- All dependencies and versions
- License information
- Security vulnerability data

SBOMs are retained for 30 days as build artifacts.

## Build Artifacts

Every build produces downloadable artifacts:

### PR Builds
- **Docker Image**: Compressed Docker image (`sidereal-pr-{PR#}-{SHA}.tar.gz`)
- **Application Snapshot**: Built application files (`sidereal-snapshot-{SHA}.tar.gz`)
- **Retention**: 7 days

### Main Branch Builds
- **SBOM**: Software Bill of Materials (`sbom-{SHA}.spdx.json`)
- **Application Snapshot**: Built application files (`sidereal-snapshot-{SHA}.tar.gz`)
- **Retention**: 30 days

### Using Artifacts

**Load Docker Image:**
```bash
# Download and load the Docker image artifact
gunzip -c sidereal-pr-123-abc123.tar.gz | docker load

# Run the image
docker run -p 5000:5000 sidereal:test-123
```

**Use Application Snapshot:**
```bash
# Extract and use the built application
tar -xzf sidereal-snapshot-abc123.tar.gz
cd dist/
node index.js
```

## Creating a Release

To create a new release:

1. **Update version** (if using package.json versioning):
   ```bash
   npm version patch  # or minor, major
   ```

2. **Create annotated tag**:
   ```bash
   git tag -a v1.0.0 -m "Release version 1.0.0"
   ```

3. **Push tag**:
   ```bash
   git push origin v1.0.0
   ```

The release workflow will automatically:
- Build and test the release
- Create Docker images with version tags
- Generate changelog from commits
- Create GitHub Release with artifacts
- Push images to container registry

## Workflow Permissions

Required repository permissions:

- **Build Workflow**:
  - `contents: read` - Checkout code
  - `packages: write` - Push to registry
  - `security-events: write` - Upload scan results

- **PR Workflow**:
  - `contents: read` - Checkout code
  - `pull-requests: write` - Comment on PRs
  - `security-events: write` - Upload scan results

- **Release Workflow**:
  - `contents: write` - Create releases
  - `packages: write` - Push to registry
  - `security-events: write` - Upload scan results

## Troubleshooting

### Build Failures

1. **TypeScript errors**: Check `npm run check` locally
2. **Build errors**: Run `npm run build` locally
3. **Docker build fails**: Test with `docker build .`

### Security Scan Failures

1. **Critical vulnerabilities**: Update base images or dependencies
2. **False positives**: Can be suppressed in `.trivyignore`

### Release Issues

1. **Invalid tag format**: Must match `v*.*.*`
2. **Push permissions**: Ensure you have write access
3. **Workflow not triggered**: Use annotated tags (`-a` flag)

## Local Development

To test workflows locally:

1. **Install act** (GitHub Actions local runner):
   ```bash
   brew install act  # macOS
   # or see https://github.com/nektos/act
   ```

2. **Run workflows**:
   ```bash
   act push                    # Test push events
   act pull_request           # Test PR events
   act workflow_dispatch      # Test manual triggers
   ```

## Monitoring

- **Workflow runs**: https://github.com/<owner>/<repo>/actions
- **Security alerts**: https://github.com/<owner>/<repo>/security
- **Package registry**: https://github.com/<owner>/<repo>/packages

## Best Practices

1. **Never commit secrets** - Use GitHub Secrets for sensitive data
2. **Test locally first** - Validate builds before pushing
3. **Use semantic versioning** - Follow v*MAJOR*.*MINOR*.*PATCH*
4. **Review security scans** - Address vulnerabilities promptly
5. **Keep workflows updated** - Use Dependabot for Action updates