D-15 is fixed. An Arabic digit used to fall through to the lexer's catch-all as
`signum ignotum: '5'` — true, but silent about what to write instead, and this
is the likeliest first mistake in a Roman-numeral language. Digits are now
matched ahead of the catch-all and told how a numeral is written.

The digit is quoted, so the robustness suite's no-Arabic-digits rule — which
strips quoted spans before checking — is satisfied.

The catch-all this case used to exercise is still reachable for genuinely
unknown characters; `errores/signum-ignotum-extra-numerum` covers it.
