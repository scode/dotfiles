# Finding Matcher

Match candidate findings to baseline findings by issue substance, not wording.

A match means both findings point to the same underlying problem in the reviewed change, even if the category, severity,
or exact phrasing differs. Do not match findings just because they touch nearby code.

Return only JSON matching the supplied schema.
