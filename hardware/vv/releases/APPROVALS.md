# Approvals

Alex's decisions, one line each. Only Alex edits and commits this file.

`check_trace.py` reads approvals from here and nowhere else: a requirement's APPROVED,
REJECTED or DEFERRED, a DFMEA row's ACCEPTED and a signed waiver each need a line here, or
the register fails its structure check. It checks who wrote each line with `git blame`: a
line counts only once it is committed, and a line written in a commit that carries a Claude
trailer (`Co-Authored-By: Claude`, `Claude-Session:`) is an error. A signed commit is better
still. A later line for the same item replaces an earlier one. A requirement's statement
edited after its approval needs a new line.

Columns:

- ID: A-NNN, the next free number.
- Item: a REQ, DF or W id.
- Decision: APPROVED, REJECTED or DEFERRED (requirements); ACCEPTED (DFMEA rows); WAIVED
  (waivers).
- Scope: for ACCEPTED and WAIVED, the last phase the decision holds for (ORDER, CASE, EVT,
  DVT, PVT, SALE); an acceptance for EVT does not open the DVT or PVT gate. For requirements,
  `-`.
- Reason: why. Required for everything but APPROVED.
- By: who decided.
- Date: YYYY-MM-DD.

Example (not a decision):
`| A-001 | DF-87 | ACCEPTED | DVT | hobby units kept by their builder; revisit before SALE | Alex | 2026-10-25 |`

| ID | Item | Decision | Scope | Reason | By | Date |
|---|---|---|---|---|---|---|
