# Open Questions: fixonce-memory-layer

## 🔴 Blocking



### Cross-Cutting / Affects Graduated Stories
[Questions that may trigger revisions to graduated stories]

### Problem Space (Phase 1)
[Questions about problem domain that may surface new stories]

---

## 🟡 Clarifying
[Questions that help but don't block progress]


---

## 🔵 Research Pending
[Questions requiring investigation]

---

## 🟠 Watching (May Affect Graduated)
*Questions that could trigger revisions:*
[Track questions that may require changes to completed stories]

---

## Question Log Summary

**Total Questions**: 0
**Open**: 0
**Resolved**: 0


<!-- Resolved: Q4 - Deferred. Single local server, no team scoping in v1. See D3. -->

<!-- Resolved: Q7 - Supabase (Postgres + pgvector) for all storage. Voyage AI voyage-code-3 for embeddings. See D8, D9. -->

<!-- Resolved: Q2 - Gate applies to AI-created memories only. Reject: vague, too specific, duplicate, obvious. Accept: actionable, generalizable, has 'why'. See D11. -->

<!-- Resolved: Q1 - Top 5 full memories + summaries/cache keys for next 10-20. Two-tier approach respects context budget. See D16. -->

<!-- Resolved: Q5 - Blocking quick check at UserPromptSubmit should be fast (sub-second). Async deep search can take longer since results are injected mid-run. SessionStart can take 1-2s since it's pre-work. See D15. -->

<!-- Resolved: Q3 - Schema finalized. See Story 1 for full schema, Story 4 for version_predicates format (D18). -->

<!-- Resolved: Q6 - Cross-project handled via project_name/repo_url/workspace_path fields (D6). Version scoping handled via version_predicates (D18). Memories with no project fields = ecosystem-wide. -->