# HelmBench Autopsy: claude-code / Native

## Summary

- Suite: `refactoringminer-git-regressions`
- Tasks: `10`
- Failed tasks: `7`
- Validation gaps: `7`
- Overbroad edits: `0`
- Missing expected inspections: `17`
- Changed without read: `0`
- High risk tasks: `7`
- Source-free: `true`

## Tasks

| Task | Status | Risk | Changed | Overbroad | Missing inspections | Validation gap |
| --- | --- | --- | ---: | ---: | ---: | --- |
| `git-regression-092c13f035f9` | Failure | High | 1 | 0 | 1 | yes |
| `git-regression-1b04d6aae2e4` | Success | Low | 2 | 0 | 0 | no |
| `git-regression-1b9f2cf08b3c` | Failure | High | 5 | 0 | 0 | yes |
| `git-regression-23e298ae221c` | Failure | High | 3 | 0 | 0 | yes |
| `git-regression-4fa3c1a48ad4` | Success | Medium | 3 | 0 | 4 | no |
| `git-regression-949bddcd3509` | Success | Medium | 2 | 0 | 1 | no |
| `git-regression-97e31265fd95` | Failure | High | 0 | 0 | 5 | yes |
| `git-regression-bd0b2277933f` | Failure | High | 4 | 0 | 1 | yes |
| `git-regression-fa29ed0c80c8` | Failure | High | 0 | 0 | 5 | yes |
| `git-regression-fa8df046b0e0` | Failure | High | 2 | 0 | 0 | yes |

### `git-regression-092c13f035f9`

- Status: `Failure`
- Risk: `High`
- Changed files: `src/main/java/gr/uom/java/xmi/decomposition/UMLOperationBodyMapper.java`
- Overbroad edits: none
- Missing expected inspections: `documentation/accuracy.md`
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.
  - Expected files were neither read nor edited.

### `git-regression-1b04d6aae2e4`

- Status: `Success`
- Risk: `Low`
- Changed files: `src/main/java/gui/MarkAsViewed.java`, `src/main/java/gui/webdiff/viewers/spv/AbstractSinglePageView.java`
- Overbroad edits: none
- Missing expected inspections: none
- Changed without recorded read: none
- Notes:
  - No source-free autopsy issues detected.

### `git-regression-1b9f2cf08b3c`

- Status: `Failure`
- Risk: `High`
- Changed files: `documentation/mcp.md`, `src/main/java/gui/webdiff/WebDiff.java`, `src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpService.java`, `src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpTools.java`, `src/main/java/org/refactoringminer/mcp/WebDiffBrowserLauncher.java`
- Overbroad edits: none
- Missing expected inspections: none
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.

### `git-regression-23e298ae221c`

- Status: `Failure`
- Risk: `High`
- Changed files: `src/main/java/gui/webdiff/WebDiff.java`, `src/main/java/org/refactoringminer/astDiff/models/DiffMetaInfo.java`, `src/main/java/org/refactoringminer/rm1/GitHistoryRefactoringMinerImpl.java`
- Overbroad edits: none
- Missing expected inspections: none
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.

### `git-regression-4fa3c1a48ad4`

- Status: `Success`
- Risk: `Medium`
- Changed files: `src/main/java/gui/webdiff/WebDiff.java`, `src/main/java/org/refactoringminer/mcp/McpDiffBrowserResult.java`, `src/main/java/org/refactoringminer/mcp/WebDiffBrowserLauncher.java`
- Overbroad edits: none
- Missing expected inspections: `docker/Dockerfile`, `docker/README.md`, `docker/native/Dockerfile-native`, `documentation/mcp.md`
- Changed without recorded read: none
- Notes:
  - Expected files were neither read nor edited.

### `git-regression-949bddcd3509`

- Status: `Success`
- Risk: `Medium`
- Changed files: `src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpService.java`, `src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpTools.java`
- Overbroad edits: none
- Missing expected inspections: `documentation/mcp.md`
- Changed without recorded read: none
- Notes:
  - Expected files were neither read nor edited.

### `git-regression-97e31265fd95`

- Status: `Failure`
- Risk: `High`
- Changed files: none
- Overbroad edits: none
- Missing expected inspections: `src/main/java/gui/webdiff/RunMode.java`, `src/main/java/gui/webdiff/dir/DirComparator.java`, `src/main/java/org/refactoringminer/astDiff/models/DiffMetaInfo.java`, `src/main/java/org/refactoringminer/astDiff/utils/URLHelper.java`, `src/main/java/org/refactoringminer/rm1/GitHistoryRefactoringMinerImpl.java`
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.
  - Expected files were neither read nor edited.

### `git-regression-bd0b2277933f`

- Status: `Failure`
- Risk: `High`
- Changed files: `build.gradle`, `src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpTools.java`, `src/main/java/org/refactoringminer/mcp/WebDiffBrowserLauncher.java`, `src/main/resources/logback.xml`
- Overbroad edits: none
- Missing expected inspections: `documentation/mcp.md`
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.
  - Expected files were neither read nor edited.

### `git-regression-fa29ed0c80c8`

- Status: `Failure`
- Risk: `High`
- Changed files: none
- Overbroad edits: none
- Missing expected inspections: `src/main/java/gui/webdiff/RunMode.java`, `src/main/java/gui/webdiff/WebDiff.java`, `src/main/java/org/refactoringminer/RefactoringMiner.java`, `src/main/java/org/refactoringminer/api/GitHistoryRefactoringMiner.java`, `src/main/java/org/refactoringminer/rm1/GitHistoryRefactoringMinerImpl.java`
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.
  - Expected files were neither read nor edited.

### `git-regression-fa8df046b0e0`

- Status: `Failure`
- Risk: `High`
- Changed files: `src/main/java/gr/uom/java/xmi/decomposition/ReplacementAlgorithm.java`, `src/main/java/gr/uom/java/xmi/decomposition/UMLOperationBodyMapper.java`
- Overbroad edits: none
- Missing expected inspections: none
- Changed without recorded read: none
- Notes:
  - Task did not end in success.
  - No successful expected validation was recorded.

## Privacy

- Raw source logged: `false`
- Raw prompts logged: `false`
- Raw transcripts logged: `false`
- Raw terminal logs logged: `false`
