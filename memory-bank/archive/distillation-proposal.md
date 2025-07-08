# Memory Bank Distillation Proposal

## Overview
Current memory bank files are 96-816 lines each with significant redundancy. Proposal to distill each file to <200 lines while preserving recent, salient information.

## File-by-File Distillation Plan

### 1. projectbrief.md (119→100 lines)
**KEEP:** Vision, aspirations, test protocol, design philosophy  
**CONDENSE:** Detailed use cases, verbose success criteria  
**MOVE TO ARCHIVE:** Historical changelog entries  

### 2. productContext.md (93→80 lines)  
**KEEP:** Problem statement, product goals, UX principles  
**CONDENSE:** Verbose problem descriptions  
**REMOVE:** Duplicate file hierarchy and changelog (already in techContext)

### 3. systemPatterns.md (390→150 lines)
**KEEP:** Core architectural patterns, registry pattern, pure functions  
**CONDENSE:** Verbose explanations, merge duplicate sections  
**MOVE TO ARCHIVE:** Detailed changelogs, historical patterns  

### 4. activeContext.md (816→180 lines)
**KEEP:** Native .sutra file loading assessment (most critical current info)  
**KEEP:** Debug files documentation  
**CONDENSE:** Pipeline assessment (move details to parsing-pipeline-plan.md)  
**REMOVE:** Duplicate information available elsewhere  

### 5. progress.md (454→150 lines)  
**KEEP:** Native file loading status, debug infrastructure, current blockers  
**CONDENSE:** Completed work summaries  
**MOVE TO ARCHIVE:** Historical roadmaps, detailed macro bootstrapping  

### 6. techContext.md (231→120 lines)
**KEEP:** Tech stack, project structure, debug infrastructure  
**CONDENSE:** Verbose explanations, duplicate architecture info  
**REMOVE:** Redundant changelog entries  

### 7. projectPatterns.md (96→60 lines)
**KEEP:** All content (already concise and relevant)  
**CONDENSE:** Minor consolidation of similar points  

## Redundancy Elimination Strategy

1. **Single Source of Truth:** File hierarchy only in techContext.md  
2. **Test Protocol:** Brief mention + reference to canonical location  
3. **Changelogs:** Major items only, details in git history  
4. **Cross-references:** Simplified and consolidated  

## Implementation Approach

1. Create archive/ subdirectory for historical content  
2. Update each file systematically  
3. Ensure no critical information is lost  
4. Maintain cross-reference integrity  

## Expected Outcome

- Total reduction from ~2200 lines to ~840 lines (62% reduction)  
- Preserved: All critical current information (native file loading, blockers, debug tools)  
- Enhanced: Focus on actionable, recent, and architectural information  
- Improved: Token efficiency and readability
