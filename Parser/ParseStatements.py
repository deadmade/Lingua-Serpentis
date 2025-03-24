from Parser.ParseDeclaration import parse_declaration
from Parser.ParseExpression import parse_function_call
from Parser.ParseLoops import parse_while_loop
from Parser.ParsePrint import parse_print


def parse_normal_statement(self, statements):
    """Parse a single statement based on token type"""
    if not self.current_token:
        return

    token_type = self.current_token[0]

    if token_type == "COMMENT":
        statements.append({"type": "comment", "value": self.current_token[1]})
    elif token_type in ["INT", "STRING", "CHAR"]:
        statements.append(parse_declaration(self, statements))
    elif token_type == "PRINT":
        statements.append(parse_print(self, statements))
    elif token_type == "IF":
        statements.append(self.parse_if_statement(statements))
    elif token_type == "WHILE":
        statements.append(parse_while_loop(self, statements))
    elif token_type == "IDENTIFIER":
        function_call, new_statements = parse_function_call(self, statements)
        statements.append(function_call)
        if new_statements:
            statements.extend(new_statements)
    elif token_type == "NEWLINE" or token_type == "SEMICOLON":
        # Skip newlines and semicolons
        pass
    else:
        # Unknown token type
        statements.append({"type": "error", "value": f"Unknown token type: {token_type}"})

    return statements


# TODO: Das hier ist noch nicht fertig
def parse_loop_statement(self, statements):
    """Parse a single statement based on token type"""
    if not self.current_token:
        return

    token_type = self.current_token[0]

    if token_type == "COMMENT":
        statements.append({"type": "comment", "value": self.current_token[1]})
    elif token_type in ["INT", "STRING", "CHAR"]:
        statements.append(parse_declaration(self, statements))
    elif token_type == "PRINT":
        statements.append(parse_print(self, statements))
    elif token_type == "IF":
        statements.append(self.parse_if_statement(statements))
    elif token_type == "WHILE":
        statements.append(parse_while_loop(self, statements))
    elif token_type == "BREAK":
        statements.append({"type": "break"})
    elif token_type == "CONTINUE":
        statements.append({"type": "continue"})
    elif token_type == "IDENTIFIER":
        function_call, new_statements = parse_function_call(self, statements)
        statements.append(function_call)
        if new_statements:
            statements.extend(new_statements)
    elif token_type == "NEWLINE" or token_type == "SEMICOLON":
        # Skip newlines and semicolons
        pass
    else:
        # Unknown token type
        statements.append({"type": "error", "value": f"Unknown token type: {token_type}"})

    return statements
