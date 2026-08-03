`expectatum: Semicolon` prints the Rust `TokenKind` Debug name — see D-02.
The line is 3, not 2: the parser reports at the token following the missing
semicolon. Pinned deliberately.
