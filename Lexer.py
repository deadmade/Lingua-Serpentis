import re

import LanguageDefinition

TOKEN_REGEX = "|".join(f"(?P<{name}>{regex})" for name, regex in LanguageDefinition.TOKEN_SPECS)


def lexer(code):
    tokens = []
    for match in re.finditer(TOKEN_REGEX, code):
        kind = match.lastgroup
        value = match.group()
        if kind != "SKIP":
            tokens.append((kind, value))
    return tokens
