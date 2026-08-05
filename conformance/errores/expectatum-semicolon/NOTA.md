D-02 is fixed. The message names the missing token as `';'` — the character the
user would have had to type — rather than `Semicolon`, the Debug rendering of a
private Rust enum. `TokenKind::name` is exhaustive with no catch-all, so adding
or renaming a variant is a compile error rather than a silently rewritten
diagnostic.

Still pinned deliberately: the reported line is **3, not 2**. The parser
notices the missing semicolon at the token that follows it, which is on the
next line. That is a separate question from D-02 and is unchanged.
