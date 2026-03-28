# Pull Request Review Policy

This document outlines the review policy for pull requests in this repository.

## Review Requirements

### Before Requesting Review
- [ ] Code compiles and builds successfully
- [ ] All tests pass locally
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Commit messages are clear
- [ ] PR description is complete

### Review Categories

#### Changes Requiring Approval
- Database schema changes
- API changes
- Security-related changes
- Performance-critical code
- Breaking changes

#### Automated Checks
- CI/CD pipeline must pass
- Code coverage threshold met
- No new security vulnerabilities
- Linting checks pass

## Reviewer Guidelines

### Response Time
- Aim to review within 24 hours
- If unavailable, assign an alternate reviewer

### Review Thoroughness
- Check for correctness and completeness
- Verify tests are adequate
- Consider edge cases
- Assess security implications
- Evaluate performance impact

### Feedback Style
- Be constructive and respectful
- Explain reasoning for suggestions
- Provide examples for unclear issues
- Acknowledge good work

## Approval Criteria

### Single Reviewer Approval
- Documentation changes
- Bug fixes (low complexity)
- Test improvements
- Refactoring (low risk)

### Two Reviewer Approval
- New features
- API changes
- Database changes
- Security changes

### Maintainer Approval
- Breaking changes
- Major architectural changes
- Deprecation of features
