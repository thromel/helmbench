# HelmBench Autopsy: claude-code / Native

## Summary

- Suite: `example-auth-bugs`
- Tasks: `1`
- Failed tasks: `1`
- Validation gaps: `1`
- Overbroad edits: `0`
- Missing expected inspections: `1`
- Changed without read: `0`
- High risk tasks: `1`
- Source-free: `true`

## Tasks

| Task | Status | Risk | Changed | Overbroad | Missing inspections | Validation gap |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `auth-redirect-001` | Failure | High | 1 | 0 | 1 | yes |

### `auth-redirect-001`

- Status: `Failure`
- Risk: `High`
- Changed files: `src/auth/session.ts`
- Overbroad edits: none
- Missing expected inspections: `src/auth/middleware.ts`
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.
  - Expected files were neither read nor edited.

## Privacy

- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
