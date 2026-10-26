# Approvals

Alex's decisions, one line each. Only Alex writes lines in the table below and commits them.

`check_trace.py` reads approvals from here and nowhere else: a requirement's APPROVED,
REJECTED or DEFERRED, a DFMEA row's ACCEPTED and a signed waiver each need a line here, or
the register fails its structure check. It finds each line's commit with `git blame`, and a
line counts only when that commit is signed with the key hardware/vv/README.md pins
('Approvals'): signing is required, not a bonus. A line not yet committed, in an unsigned
commit, or signed with any other key (another key in the allowed-signers file included) is
pending and counts at no gate; a line written in a merge commit, or in a commit that carries
a Claude trailer (`Co-Authored-By: Claude`, `Claude-Session:`), is an error. A later line for
the same item replaces an earlier one. A requirement's statement edited after its approval
needs a new line.

Bulk lines save signing 130 requirement lines one by one: `REQ-ALL@<commit>` with APPROVED
approves every requirement whose statement is the same as at that commit, and `DF-H@<commit>`
with ACCEPTED and a scope accepts every H row whose failure mode, effect and severity are the
same as at that commit. Each ID is still checked on its own: one edited after the commit is
not covered and needs its own line.

Columns:

- ID: A-NNN, the next free number.
- Item: a REQ, DF or W id.
- Decision: APPROVED, REJECTED or DEFERRED (requirements); ACCEPTED (DFMEA rows); WAIVED
  (waivers).
- Scope: for ACCEPTED, WAIVED and DEFERRED, the last phase the decision holds for (ORDER,
  CASE, EVT, DVT, PVT, SALE); an acceptance for EVT does not open the DVT or PVT gate, and a
  requirement deferred to EVT is PROPOSED again at DVT. For APPROVED and REJECTED, `-`.
- Reason: why. Required for everything but APPROVED.
- By: who decided.
- Date: YYYY-MM-DD.

Examples (not decisions):
`| A-001 | DF-87 | ACCEPTED | DVT | hobby units kept by their builder; revisit before SALE | Alex | 2026-10-25 |`
`| A-002 | REQ-PRD-03.R | DEFERRED | EVT | set once the cell's spec sheet and a capacity sample exist | Alex | 2026-10-25 |`
`| A-003 | REQ-ALL@<commit> | APPROVED | - | every requirement as worded at that commit | Alex | 2026-10-27 |`

| ID | Item | Decision | Scope | Reason | By | Date |
|---|---|---|---|---|---|---|
