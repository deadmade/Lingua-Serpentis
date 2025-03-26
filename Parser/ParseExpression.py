from Parser.HelperFunctions import peek_next_token, handle_numbers, advance_token, conditions_mapping, \
    get_operator_precedence


def parse_expression(self, statements):
    """Parse an expression (number, identifier, function call, operations)"""
    if not self.current_token:
        return None, None, None

    token_type = self.current_token[0]
    next_token = peek_next_token(self)

    # Handle parenthesized expressions
    if token_type == "LPAREN":
        advance_token(self)  # Move past LPAREN
        inner_expr, inner_operation_type, inner_statements = parse_expression(self, statements)

        if not self.current_token or self.current_token[0] != "RPAREN":
            return {"type": "error", "value": "Missing closing parenthesis"}, None, None

        advance_token(self)  # Move past RPAREN
        left_expr = inner_expr
        left_type = "parenthesized"
        stored_operation_type = inner_operation_type  # Store for later use

    # Handle number literal
    elif token_type == "NUMBER":
        left_expr = {"type": "number", "value": handle_numbers(self)}
        left_type = "number"
        advance_token(self)
        stored_operation_type = None

    # Handle string literals
    elif token_type == "STRING_LITERAL":
        left_expr = {"type": "string", "value": self.current_token[1]}
        left_type = "string"
        advance_token(self)
        stored_operation_type = None

    # Handle identifiers and function calls
    elif token_type == "IDENTIFIER":
        identifier = self.current_token[1]
        advance_token(self)

        # Check if this is a function call
        if self.current_token and self.current_token[0] == "LPAREN":
            self.current_token_index -= 1  # Go back to parse the function call properly
            self.current_token = self.tokens[self.current_token_index]
            function_call = parse_function_call(self, statements)
            return function_call, "declaration_function_call", None

        left_expr = {"type": "identifier", "value": identifier}
        left_type = "identifier"
        stored_operation_type = None

    else:
        return None, None, None

    # Check if there's an operation following the parsed expression
    if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV", "ISNOTEQUAL", "ISGREATER",
                                                        "ISLESS"]:
        return parse_operation(self, left_expr, left_type, statements)

    # No operation, return the expression as is
    operation_type = "declaration_assignment"
    if left_type == "parenthesized" and stored_operation_type:
        operation_type = stored_operation_type

    return left_expr, operation_type, None


def parse_operation(self, left_expr, left_type, statements):
    """Parse binary operations with precedence handling"""
    # Get the operation symbol
    current_op = self.current_token[0]
    condition = conditions_mapping(current_op)
    advance_token(self)

    # Parse the right side of the operation
    if not self.current_token:
        return {"type": "error", "value": "Expected expression after operator"}, None, None

    # Parse right operand
    right_expr = None
    if self.current_token[0] == "NUMBER":
        right_expr = {"type": "number", "value": handle_numbers(self)}
        advance_token(self)
    elif self.current_token[0] == "IDENTIFIER":
        right_expr = {"type": "identifier", "value": self.current_token[1]}
        advance_token(self)
    elif self.current_token[0] == "LPAREN":
        # Handle parenthesized right expression
        advance_token(self)  # Move past LPAREN
        right_expr, _, inner_statements = parse_expression(self, statements)

        if not self.current_token or self.current_token[0] != "RPAREN":
            return {"type": "error", "value": "Missing closing parenthesis"}, None, None

        advance_token(self)  # Move past RPAREN
    else:
        return {"type": "error", "value": "Invalid right operand in operation"}, None, None

    # Check for next operation
    if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV", "ISNOTEQUAL", "ISGREATER",
                                                        "ISLESS"]:
        next_op = self.current_token[0]

        # Check precedence of current and next operator
        current_precedence = get_operator_precedence(current_op)
        next_precedence = get_operator_precedence(next_op)

        if next_precedence > current_precedence:
            # Higher precedence operator, parse it first
            temp_right, _, _ = parse_operation(self, right_expr, "expression", statements)

            # Create operation with the correctly nested right side
            operation = {
                "type": "operation",
                "value": left_expr["value"] if isinstance(left_expr, dict) and "value" in left_expr else left_expr,
                "condition": condition,
                "value2": temp_right
            }

            return operation, "declaration_operation", None
        else:
            # Equal or lower precedence, create current operation first
            operation = {
                "type": "operation",
                "value": left_expr["value"] if isinstance(left_expr, dict) and "value" in left_expr else left_expr,
                "condition": condition,
                "value2": right_expr["value"] if isinstance(right_expr, dict) and "value" in right_expr else right_expr
            }

            # Continue parsing with this operation as the left side
            return parse_operation(self, operation, "operation", statements)

    # No more operations, create and return the current operation
    operation = {
        "type": "operation",
        "value": left_expr["value"] if isinstance(left_expr, dict) and "value" in left_expr else left_expr,
        "condition": condition,
        "value2": right_expr["value"] if isinstance(right_expr, dict) and "value" in right_expr else right_expr
    }

    return operation, "declaration_operation", None


def parse_function_call(self, statements):
    """Parse a function call with arguments"""
    function_name = self.current_token[1]
    advance_token(self)  # Move to LPAREN

    if not self.current_token or self.current_token[0] != "LPAREN":
        return {"type": "error", "value": f"Expected '(' after function name {function_name}"}, None, None

    advance_token(self)  # Move past LPAREN

    arguments = []
    # Parse arguments until we find the closing parenthesis
    while self.current_token and self.current_token[0] != "RPAREN":
        arg, _, _ = parse_expression(self, statements)
        if arg:
            arguments.append(arg)

        # Skip commas between arguments
        if self.current_token and self.current_token[0] == "COMMA":
            advance_token(self)

    if not self.current_token or self.current_token[0] != "RPAREN":
        return {"type": "error", "value": f"Expected ')' to close function call to {function_name}"}, None, None

    advance_token(self)  # Move past RPAREN

    function_call = {
        "type": "function_call",
        "name": function_name,
        "arguments": arguments
    }

    # Check if there's an operation following the function call
    if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV", "ISNOTEQUAL", "ISGREATER",
                                                        "ISLESS"]:
        return parse_operation(self, function_call, "function_call", statements)

    return function_call
