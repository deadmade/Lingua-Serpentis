from Parser.HelperFunctions import peek_next_token, handle_numbers, advance_token, conditions_mapping


def parse_expression(self, statements):
    """Parse an expression (number, identifier, function call)"""
    if not self.current_token:
        return None, None, None

    token_type = self.current_token[0]
    next_token = peek_next_token(self)

    # Handle number literal
    if token_type == "NUMBER":
        value = handle_numbers(self)
        advance_token(self)

        # Check if this is part of an operation
        if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV", "ISNOTEQUAL", "ISGREATER",
                                                            "ISLESS"]:
            condition = conditions_mapping(self.current_token[0])
            advance_token(self)

            # Parse the right side of the operation
            if self.current_token and self.current_token[0] == "NUMBER":
                value2 = handle_numbers(self)
                advance_token(self)
            elif self.current_token and self.current_token[0] == "IDENTIFIER":
                value2 = self.current_token[1]
                self.advance_token()
            else:
                value2 = None

            return {"type": "operation", "value": value, "condition": condition,
                    "value2": value2}, "declaration_operation", None

        return {"type": "number", "value": value}, "declaration_assignment", None

    # Handle string literals
    elif token_type == "STRING_LITERAL":
        value = self.current_token[1]
        advance_token(self)
        return {"type": "string", "value": value}, "declaration_assignment", None

    # Handle identifiers
    elif token_type == "IDENTIFIER":
        identifier = self.current_token[1]
        advance_token(self)

        # Check if this is a function call
        if self.current_token and self.current_token[0] == "LPAREN":
            self.current_token_index -= 1  # Go back to parse the function call properly
            self.current_token = self.tokens[self.current_token_index]
            function_call, statements_added = parse_function_call(self, statements)
            return function_call, "declaration_function_call", statements_added

        # Check if this is part of an operation
        if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV", "ISNOTEQUAL",
                                                            "ISGREATER",
                                                            "ISLESS"]:
            condition = conditions_mapping(self.current_token[0])
            advance_token(self)

            # Parse the right side of the operation
            if self.current_token and self.current_token[0] == "NUMBER":
                value2 = handle_numbers(self)
                advance_token(self)
            elif self.current_token and self.current_token[0] == "IDENTIFIER":
                value2 = self.current_token[1]
                advance_token(self)
            else:
                value2 = None

            return {"type": "operation", "value": identifier, "condition": condition,
                    "value2": value2}, "declaration_operation", None

        return {"type": "identifier", "value": identifier}, "declaration_assignment", None

    return None, None, None


def parse_function_call(self, statements):
    """Parse a function call with arguments"""
    function_name = self.current_token[1]
    advance_token(self)  # Move to LPAREN

    new_statements = []

    if not self.current_token or self.current_token[0] != "LPAREN":
        new_statements.append({"type": "error", "value": f"Expected '(' after function name {function_name}"})
        return None, new_statements

    advance_token(self)  # Move past LPAREN

    arguments = []
    # Parse arguments until we find the closing parenthesis
    while self.current_token and self.current_token[0] != "RPAREN":
        arg, _, statements = parse_expression(self, statements)
        if arg:
            arguments.append(arg)

        # Skip commas between arguments
        if self.current_token and self.current_token[0] == "COMMA":
            advance_token(self)

    if not self.current_token or self.current_token[0] != "RPAREN":
        new_statements.append(
            {"type": "error", "value": f"Expected ')' to close function call to {function_name}"})
        return None, new_statements

    advance_token(self)  # Move past RPAREN

    return {
        "type": "function_call",
        "name": function_name,
        "arguments": arguments
    }, new_statements
