def parse_if_statement(self, statements):
    """Parse an if statement with condition and body"""
    self.advance_token()  # Move past IF

    # Parse condition
    if not self.current_token or self.current_token[0] != "LPAREN":
        statements.append({"type": "error", "value": "Expected '(' after if keyword"})
        return statements

    self.advance_token()  # Move past LPAREN

    # Parse the condition expression
    condition = self.parse_expression()
    self.advance_token()

    if not self.current_token or self.current_token[0] != "RPAREN":
        statements.append({"type": "error", "value": "Expected ')' to close if condition"})
        return statements

    self.advance_token()  # Move past RPAREN

    # Parse body (statements between { and })
    if not self.current_token or self.current_token[0] != "LBRACE":
        statements.append({"type": "error", "value": "Expected '{' after if condition"})
        return statements

    self.advance_token()  # Move past LBRACE

    # Parse if body statements
    if_body = self.parse_block()

    # Check for else statement
    else_body = None
    if self.current_token and self.current_token[0] == "ELSE":
        self.advance_token()  # Move past ELSE

        # Check if it's an else if or a regular else
        if self.current_token and self.current_token[0] == "IF":
            # For else if, recursively parse another if statement
            else_body = [self.parse_if_statement()]
        else:
            # For regular else, parse the block
            if not self.current_token or self.current_token[0] != "LBRACE":
                statements.append({"type": "error", "value": "Expected '{' after else keyword"})
                return statements

            self.advance_token()  # Move past LBRACE
            else_body = self.parse_block()

    # Create and add the if statement to our statements list
    if_statement = {
        "type": "if_statement",
        "condition": condition,
        "if_body": if_body,
        "else_body": else_body
    }

    statements.append(if_statement)
    return statements
