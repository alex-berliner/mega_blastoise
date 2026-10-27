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

A third bulk line freezes the register's gate state: `GATE@<commit>` with FROZEN and a phase
scope holds every check's phase, every defect's Gate, every criterion's Needs and every
requirement's Phase and Verified-by as they stand at that commit. The newest committed GATE
line wins, and anything moved later than it fails the order gate (the gate result G1). It
covers only what is unchanged since the commit, so a row edited afterwards needs its own
line. The line's phase scope is the last phase the freeze holds for, like an acceptance's.

There is no exception line. Nothing in this file can grant a gate a pass that a signature does
not: the signing key's type is checked at the order gate, and `--accept-software-key` has to
be copied into the release record's signing section, where it is visible. A line whose Item
starts with `KEY@` (a fingerprint) is not a decision at all: it is warned about and ignored,
so a line that tries to grant the software-key exception does not count, and the order gate
still fails K1 (O-R12).

Columns:

- ID: A-NNN, the next free number.
- Item: a REQ, DF or W id.
- Decision: APPROVED, REJECTED or DEFERRED (requirements); ACCEPTED (DFMEA rows); WAIVED
  (waivers); FROZEN (a GATE@ bulk line: the gate state as it stands at a commit).
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
`| A-004 | GATE@<commit> | FROZEN | EVT | every check's phase, defect's Gate, criterion's Needs and requirement's Phase as they stand at that commit | Alex | 2026-10-27 |`

| ID | Item | Decision | Scope | Reason | By | Date |
|---|---|---|---|---|---|---|
