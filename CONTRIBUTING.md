# Contributing to Sidereal

Thank you for your interest in contributing to Sidereal! We welcome contributions from the community to help make this astrophotography management tool even better.

## 🚀 Quick Start

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/Sidereal.git
   cd Sidereal
   ```
3. **Install** dependencies:
   ```bash
   npm install
   ```
4. **Create** a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```
5. **Make** your changes
6. **Test** your changes
7. **Submit** a pull request

## 📋 Types of Contributions

We welcome several types of contributions:

### 🐛 Bug Reports
- Use the GitHub issue tracker
- Include detailed reproduction steps
- Provide environment information (OS, Node.js version, etc.)
- Include logs or screenshots if applicable

### 💡 Feature Requests
- Use GitHub Discussions for feature ideas
- Describe the problem you're trying to solve
- Explain your proposed solution
- Consider the impact on existing users

### 🔧 Code Contributions
- Bug fixes
- New features
- Performance improvements
- Documentation improvements
- Test coverage improvements

### 📖 Documentation
- README improvements
- Code comments
- API documentation
- Deployment guides
- Troubleshooting guides

## 🛠️ Development Setup

### Prerequisites
- Node.js 20+
- npm 10+
- Git
- Docker (optional, for database)

### Environment Setup
```bash
# Clone and install
git clone https://github.com/YOUR_USERNAME/Sidereal.git
cd Sidereal
npm install

# Setup environment
cp .env.example .env.local
# Edit .env.local with your development settings

# Initialize database
npm run db:generate
npm run db:migrate

# Start development server
npm run dev
```

### Development Commands
```bash
# Development
npm run dev            # Start backend server
npm run dev:watch      # Start with file watching
npm run dev:worker     # Start worker process
npm run dev:all        # Start backend + worker

# Building
npm run build          # Build for production
npm run check          # TypeScript type checking

# Database
npm run db:generate    # Generate migrations
npm run db:migrate     # Apply migrations
npm run db:studio      # Open database GUI

# Code Quality
npm run lint           # ESLint checking
npm run format         # Prettier formatting
npm run test           # Run tests
```

## 📝 Code Standards

### TypeScript
- Use strict TypeScript settings
- Provide proper type definitions
- Avoid `any` types when possible
- Use Zod schemas for validation

### Code Style
- Follow ESLint and Prettier configurations
- Use meaningful variable and function names
- Write clear, concise comments
- Keep functions small and focused

### Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add plate solving progress tracking
fix: resolve thumbnail loading issues
docs: update deployment instructions
refactor: improve database connection handling
test: add unit tests for image processing
```

### File Organization
- Place components in `apps/client/src/components/`
- API routes go in `apps/server/src/routes/`
- Shared types in `packages/shared/src/types/`
- Database schemas in `packages/shared/src/db/`

## 🧪 Testing

### Running Tests
```bash
# Run all tests
npm run test

# Run tests in watch mode
npm run test:watch

# Run specific test file
npm run test -- --testNamePattern="image processing"
```

### Writing Tests
- Write tests for new features
- Include edge cases and error conditions
- Test both success and failure scenarios
- Mock external dependencies (APIs, databases)

## 📦 Pull Request Process

### Before Submitting
1. **Test** your changes thoroughly
2. **Run** code quality checks:
   ```bash
   npm run lint
   npm run format
   npm run check
   npm run test
   ```
3. **Update** documentation if needed
4. **Add** tests for new functionality

### Pull Request Guidelines
- Use a clear, descriptive title
- Reference related issues with `Fixes #123`
- Provide a detailed description of changes
- Include screenshots for UI changes
- Keep PRs focused and atomic

### PR Template
```markdown
## Summary
Brief description of the changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed

## Screenshots (if applicable)
[Add screenshots here]

## Checklist
- [ ] Code follows project standards
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
```

## 🏗️ Architecture Guidelines

### Database Changes
- Use Drizzle ORM for database operations
- Create migrations for schema changes
- Test migrations on sample data
- Consider backward compatibility

### API Design
- Follow RESTful conventions
- Use proper HTTP status codes
- Implement proper error handling
- Include request/response validation

### Frontend Components
- Use shadcn/ui components when possible
- Follow React best practices
- Implement proper error boundaries
- Use TypeScript for all components

### Worker Processes
- Handle errors gracefully
- Implement proper logging
- Use queues for background tasks
- Consider resource limitations

## 🔒 Security Guidelines

- Never commit secrets or API keys
- Use environment variables for configuration
- Validate all user inputs
- Implement proper authentication
- Follow OWASP security guidelines

## 📚 Resources

### Documentation
- [Project README](README.md)
- [Docker Documentation](docker/README.md)
- [API Documentation](docs/api.md)

### External Resources
- [Node.js Best Practices](https://nodejs.dev/en/learn/)
- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
- [Drizzle ORM Docs](https://orm.drizzle.team/)

## 🤝 Community

### Code of Conduct
- Be respectful and inclusive
- Welcome newcomers
- Help others learn
- Keep discussions constructive

### Getting Help
- Join discussions in GitHub Issues
- Ask questions in pull requests
- Check existing documentation first
- Provide context when asking for help

### Communication
- Use clear, descriptive language
- Be patient with responses
- Provide helpful feedback
- Acknowledge contributions

## 🎯 Development Priorities

### High Priority
- Bug fixes affecting core functionality
- Security improvements
- Performance optimizations
- Documentation improvements

### Medium Priority
- New features with clear use cases
- UI/UX improvements
- Integration enhancements
- Testing improvements

### Low Priority
- Nice-to-have features
- Code refactoring
- Experimental features
- Tool improvements

## 📄 License

By contributing to Sidereal, you agree that your contributions will be licensed under the MIT License.

## 🙏 Recognition

Contributors are recognized in:
- GitHub contributors list
- Release notes for significant contributions
- README acknowledgments
- Community shout-outs

Thank you for contributing to Sidereal! Your help makes this project better for the entire astrophotography community.