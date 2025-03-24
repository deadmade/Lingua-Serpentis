import re
import LanguageDefinition

TOKEN_REGEX = "|".join(f"(?P<{name}>{regex})" for name, regex in LanguageDefinition.TOKEN_SPECS)
WRONG_TOKEN_REGEX = "|".join(LanguageDefinition.Wrong_Token_SPECS)

def lexer(code):
    tokens = []
    line_number = 1
    for match in re.finditer(TOKEN_REGEX, code):
        kind = match.lastgroup
        value = match.group()
        if kind == "NEWLINE":
            line_number += 1
        if kind != "SKIP":
            tokens.append((kind, value, line_number))

    for match in re.finditer(WRONG_TOKEN_REGEX, code):
        value = match.group()
        line_number = code[:match.start()].count('\n') + 1
        print(f"Error: Irregular token '{value}' found on line {line_number}")
        return None

    return tokens
