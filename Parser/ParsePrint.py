def parse_print(self, statements):
    """Parse a print statement"""
    self.advance_token()  # Move past PRINT

    if not self.current_token or self.current_token[0] != "LPAREN":
        statements.append({"type": "error", "value": "Expected '(' after print"})
        return statements

    self.advance_token()  # Move past LPAREN

    expression = self.parse_expression()

    if not self.current_token or self.current_token[0] != "RPAREN":
        statements.append({"type": "error", "value": "Expected ')' to close print statement"})
        return statements

    self.advance_token()  # Move past RPAREN

    statements.append({
        "type": "print",
        "expression": expression
    })

    return statements
