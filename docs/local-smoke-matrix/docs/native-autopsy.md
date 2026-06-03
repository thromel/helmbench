# HelmBench Autopsy: demo-baseline / Native

## Summary

- Suite: `local-run-smoke`
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
| `demo-auth-001` | Failure | High | 0 | 0 | 1 | yes |

### `demo-auth-001`

- Status: `Failure`
- Risk: `High`
- Changed files: none
- Overbroad edits: none
- Missing expected inspections: `examples/demo-app/auth.txt`
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
