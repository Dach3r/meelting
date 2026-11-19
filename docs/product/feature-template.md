# Feature format and decision-making

## Product Owner responsibilities

1. Keep decisions aligned with the vision.
2. Identify real user problems before proposing features.
3. Define user stories and acceptance criteria.
4. Prioritize the backlog by value, effort, risk and dependencies.
5. Keep the MVP scope under control.
6. Distinguish confirmed requirements, proposals and hypotheses.
7. Identify dependencies between features.
8. Validate feasibility with technical agents.
9. Ask for clarification on important undefined decisions.
10. Prevent the technical implementation from displacing the product goals.

## Feature template

```markdown
# <Name>

**Problem:** the user's concrete need.
**Goal:** the outcome we are after.
**User story:** As a [type of user], I want [action], so that [benefit].

## Functional flow
1. ...

## Acceptance criteria
- [ ] Verifiable condition.

## Business rules
- System constraints and behaviors.

## Dependencies
- Required features or components.

**Priority:** Must Have | Should Have | Could Have | Won't Have
**Success metric:** how we will validate its usefulness.
```

## Decision-making rules

Before proposing a feature, assess:

- What concrete problem does it solve?
- Is it needed for the MVP?
- What value does it bring to the user?
- Can it be implemented in a simpler way?
- Does it depend on capabilities that do not exist yet?
- How will we know whether it works?
- Does it introduce security, permission or privacy risks?
- Are we validating a need or adding complexity?

If a product decision is missing: present alternatives, advantages, risks and a justified recommendation. Do not assume important decisions without validation from the product owner.
