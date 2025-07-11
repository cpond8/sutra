# Dev Log

<!-- RULES ──────────────────────────────────────────────────────────────
1.  A “pulse” is a single line inserted right AFTER the PULSE_START tag:

    [YYYY-MM-DD HH:MM] text…          ← normal note
    [YYYY-MM-DD HH:MM] ！ text…        ← irreversible decision (full-width exclam)

2.  When the user says **end session**:
    a.  Summarise ALL lines between PULSE_START and PULSE_END
        into ≤ 6 Markdown bullets.  Preserve any “！” by prefixing
        the bullet with the same mark.
    b.  Insert that summary block directly AFTER SESSIONS_START in
        the form:
           ### YYYY-MM-DD HH:MM–HH:MM
           • …

    c.  Keep only the 5 most-recent session blocks.  If > 5, move the
        oldest ones (unchanged) to ARCHIVE_START.
    d.  Delete every line between PULSE_START and PULSE_END so the
        next session starts clean.
----------------------------------------------------------------------- -->

## Pulse
<!-- PULSE_START -->
[2025-07-11 13:27] Migration review: Ordered remaining tasks by difficulty (tests/docs first, then batch type refactors, then macro/clone audits, integration, and benchmarking) per user request.
[2025-07-11 13:16] Ongoing AstNode/Arc migration: easy wins done, pattern matching & clone audit next.
<!-- PULSE_END -->

## Sessions
<!-- SESSIONS_START -->

<!-- SESSIONS_END -->

## Archive (older than 5 sessions)
<!-- ARCHIVE_START -->

<!-- ARCHIVE_END -->
