# Security Policy

## Supported Versions

Security fixes are provided for the default branch.

## Reporting a Vulnerability

- Do not open public issues for security reports.
- Email the maintainer directly with:
  - A clear description of the issue
  - Reproduction steps or proof of concept
  - Impact assessment
  - Suggested remediation (if available)

The maintainer will acknowledge receipt within 72 hours and provide status updates as triage progresses.

## Handling Secrets

- Never commit secrets to the repository.
- Use environment variables for credentials (`JWT_SECRET`, Stripe keys, SMTP secrets).
- Rotate affected credentials immediately if exposure is suspected.
