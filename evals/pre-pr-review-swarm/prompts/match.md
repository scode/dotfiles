# Finding Matcher

Match candidate findings to baseline findings by issue substance, not wording.

A match means both findings point to the same underlying problem in the reviewed change, even if the category, severity,
or exact phrasing differs. Do not match findings just because they touch nearby code.

Each finding id may appear in at most one match. Runs repeat several times per side, so the same underlying issue often
appears under multiple ids; when a finding has several plausible counterparts, pick the single best one and leave the
rest unmatched. Do not emit many-to-one matches — they are rejected and fail the whole comparison.

Return only JSON matching the supplied schema.
