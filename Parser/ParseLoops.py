from Parser.HelperFunctions import advance_token, parse_block
from Parser.ParseExpression import parse_expression


def parse_while_loop(self, statements):
    """Parse a while loop with condition and body"""
    advance_token(self)  # Move past WHILE

    new_statements = []

    if not self.current_token or self.current_token[0] != "LPAREN":
        new_statements.append({"type": "error", "value": "Expected '(' after while keyword"})
        return new_statements

    advance_token(self)  # Move past LPAREN
    condition = parse_expression(self, new_statements)

    if not self.current_token or self.current_token[0] != "RPAREN":
        new_statements.append({"type": "error", "value": "Expected ')' to close while condition"})
        return new_statements

    advance_token(self)  # Move past RPAREN

    if not self.current_token or self.current_token[0] != "LBRACE":
        new_statements.append({"type": "error", "value": "Expected '{' after while condition"})
        return new_statements

    advance_token(self)  # Move past LBRACE
    body_statements = parse_block(self, statements)

    statements.append({
        "type": "while_loop",
        "condition": condition,
        "body": body_statements
    })

    return statements
