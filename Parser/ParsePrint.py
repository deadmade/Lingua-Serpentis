from Parser.HelperFunctions import advance_token, peek_next_token
from Parser.ParseExpression import parse_expression


def parse_print(self, statements):
    """Parse a print statement"""
    advance_token(self)  # Move past PRINT

    if not self.current_token or self.current_token[0] != "LPAREN":
        return {"type": "error", "value": "Expected '(' after print"}

    advance_token(self)  # Move past LPAREN

    expression = parse_expression(self, statements)

    if not self.current_token or self.current_token[0] != "RPAREN":
        return {"type": "error", "value": "Expected ')' to close print statement"}

    advance_token(self)  # Move past RPAREN

    return {
        "type": "print",
        "expression": expression
    }

