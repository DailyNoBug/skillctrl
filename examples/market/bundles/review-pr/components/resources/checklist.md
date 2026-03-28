# Pull Request Review Checklist

Use this checklist when reviewing pull requests.

## General
- [ ] PR title is clear and descriptive
- [ ] PR description explains the what and why
- [ ] Related issues/tickets are linked
- [ ] Breaking changes are documented
- [ ] Migration guide provided if needed

## Code Quality
- [ ] Code follows project style guidelines
- [ ] Code is well-structured and readable
- [ ] Complex logic has comments
- [ ] No dead or commented code
- [ ] No console.log or debug statements
- [ ] Error handling is appropriate

## Testing
- [ ] New tests added
- [ ] Existing tests updated
- [ ] Edge cases covered
- [ ] Error cases tested
- [ ] Integration tests included if needed

## Documentation
- [ ] README updated if needed
- [ ] API documentation updated
- [ ] Comments added for complex code
- [ ] Usage examples provided
- [ ] Changelog updated

## Security
- [ ] No hardcoded secrets
- [ ] Input validation present
- [ ] Output encoding correct
- [ ] Authentication/authorization checked
- [ ] SQL injection prevented
- [ ] XSS prevention in place

## Performance
- [ ] No obvious performance issues
- [ ] Database queries optimized
- [ ] Caching used where appropriate
- [ ] No memory leaks
- [ ] Async operations correct

## Compatibility
- [ ] Backward compatible (or documented if not)
- [ ] Works on supported platforms
- [ ] Dependencies updated safely
- [ ] Deprecation warnings added if needed

## Deployment
- [ ] Migration scripts included if needed
- [ ] Environment variables documented
- [ ] Configuration changes noted
- [ ] Deployment steps documented
