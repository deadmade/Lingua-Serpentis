import RomanNumeralConverter

class Parser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.current_token_index = 0
        self.current_token = None
        self.c_code = []
        self.functions = {}
        self.statements = []

        # Initialize with the first token if available
        if tokens:
            self.current_token = tokens[0]

    def parse(self):
        """Parse all tokens and return the generated C code"""
        while self.current_token_index < len(self.tokens):
            self.parse_statement()
            self.advance_token()

        # Convert statements to C code
        print(self.statements)
        self.generate_c_code()
        return "\n".join(self.c_code)

    def advance_token(self):
        """Move to the next token"""
        self.current_token_index += 1
        if self.current_token_index < len(self.tokens):
            self.current_token = self.tokens[self.current_token_index]
        else:
            self.current_token = None

    def parse_statement(self):
        """Parse a single statement based on token type"""
        if not self.current_token:
            return

        token_type = self.current_token[0]

        if token_type == "COMMENT":
            self.statements.append({"type": "comment", "value": self.current_token[1]})
        elif token_type in ["INT", "STRING", "CHAR"]:
            self.parse_declaration()
        elif token_type == "PRINT":
            self.parse_print()
        elif token_type == "IF":
            self.parse_if_statement()
        elif token_type == "IDENTIFIER":
            self.parse_assignment_or_function_call()
        elif token_type == "NEWLINE" or token_type == "SEMICOLON":
            # Skip newlines and semicolons
            pass
        else:
            # Unknown token type
            self.statements.append({"type": "error", "value": f"Unknown token type: {token_type}"})

    def parse_declaration(self):
        """Parse variable declaration and assignment"""
        data_type = self.datatype_mapping(self.current_token[0])
        self.advance_token()

        if not self.current_token or self.current_token[0] != "IDENTIFIER":
            self.statements.append({"type": "error", "value": "Expected identifier after type declaration"})
            return

        identifier = self.current_token[1]
        self.advance_token()

        # Simple declaration without assignment
        if not self.current_token or self.current_token[0] in ["NEWLINE", "SEMICOLON"]:
            self.statements.append({
                "type": "declaration",
                "data_type": data_type,
                "identifier": identifier
            })
            return

        # Declaration with assignment
        if self.current_token[0] == "ASSIGN":
            self.advance_token()
            value = self.parse_expression()

            self.statements.append({
                "type": "declaration_assignment",
                "data_type": data_type,
                "identifier": identifier,
                "value": value
            })

    def parse_expression(self):
        """Parse an expression (number, identifier, function call)"""
        if not self.current_token:
            return None

        token_type = self.current_token[0]

        if token_type == "NUMBER":
            value = self.handle_numbers()
            self.advance_token()
            return {"type": "number", "value": value}

        elif token_type == "STRING_LITERAL":
            value = self.current_token[1]
            self.advance_token()
            return {"type": "string", "value": value}

        elif token_type == "IDENTIFIER":
            if self.peek_next_token()[0] == "LPAREN":
                return self.parse_function_call()
            else:
                value = self.current_token[1]
                self.advance_token()
                condition = self.conditions_mapping(self.current_token[0])
                self.advance_token()
                if self.current_token[0] == "NUMBER":
                    value2 = RomanNumeralConverter.convert_roman_to_decimal( self.current_token[1])
                else:
                    value2 = self.current_token[1]
                return {"type": "identifier", "value": value, "condition": condition, "value2": value2}

        return None


    def parse_function_call(self):
        """Parse a function call with arguments"""
        function_name = self.current_token[1]
        self.advance_token()  # Move to LPAREN

        if not self.current_token or self.current_token[0] != "LPAREN":
            self.statements.append({"type": "error", "value": f"Expected '(' after function name {function_name}"})
            return None

        self.advance_token()  # Move past LPAREN

        arguments = []
        # Parse arguments until we find the closing parenthesis
        while self.current_token and self.current_token[0] != "RPAREN":
            arg = self.parse_expression()
            if arg:
                arguments.append(arg)

            # Skip commas between arguments
            if self.current_token and self.current_token[0] == "COMMA":
                self.advance_token()

        if not self.current_token or self.current_token[0] != "RPAREN":
            self.statements.append({"type": "error", "value": f"Expected ')' to close function call to {function_name}"})
            return None

        self.advance_token()  # Move past RPAREN

        return {
            "type": "function_call",
            "name": function_name,
            "arguments": arguments
        }

    def parse_print(self):
        """Parse a print statement"""
        self.advance_token()  # Move past PRINT

        if not self.current_token or self.current_token[0] != "LPAREN":
            self.statements.append({"type": "error", "value": "Expected '(' after print"})
            return

        self.advance_token()  # Move past LPAREN

        expression = self.parse_expression()

        if not self.current_token or self.current_token[0] != "RPAREN":
            self.statements.append({"type": "error", "value": "Expected ')' to close print statement"})
            return

        self.advance_token()  # Move past RPAREN

        self.statements.append({
            "type": "print",
            "expression": expression
        })

    def parse_if_statement(self):
        """Parse an if statement with condition and body"""
        self.advance_token()  # Move past IF

        # Parse condition
        if not self.current_token or self.current_token[0] != "LPAREN":
            self.statements.append({"type": "error", "value": "Expected '(' after if keyword"})
            return

        self.advance_token()  # Move past LPAREN

        # Parse the condition expression
        condition = self.parse_expression()
        self.advance_token()

        if not self.current_token or self.current_token[0] != "RPAREN":
            self.statements.append({"type": "error", "value": "Expected ')' to close if condition"})
            return

        self.advance_token()  # Move past RPAREN

        # Parse body (statements between { and })
        if not self.current_token or self.current_token[0] != "LBRACE":
            self.statements.append({"type": "error", "value": "Expected '{' after if condition"})
            return

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
                    self.statements.append({"type": "error", "value": "Expected '{' after else keyword"})
                    return

                self.advance_token()  # Move past LBRACE
                else_body = self.parse_block()

        # Create and add the if statement to our statements list
        if_statement = {
            "type": "if_statement",
            "condition": condition,
            "if_body": if_body,
            "else_body": else_body
        }

        self.statements.append(if_statement)

    def parse_block(self):
        """Parse a block of statements enclosed in braces"""
        block_statements = []

        # Parse statements until we find the closing brace
        while self.current_token and self.current_token[0] != "RBRACE":
            start_index = self.current_token_index
            self.parse_statement()

            # If we didn't advance, manually advance to avoid infinite loop
            if self.current_token_index == start_index:
                self.advance_token()

            # Get the last parsed statement if available
            if self.statements:
                block_statements.append(self.statements.pop())

        if not self.current_token or self.current_token[0] != "RBRACE":
            self.statements.append({"type": "error", "value": "Expected '}' to close block"})
            return block_statements

        self.advance_token()  # Move past RBRACE
        return block_statements

    def parse_assignment_or_function_call(self):
        """Parse either an assignment or a function call"""
        identifier = self.current_token[1]
        self.advance_token()

        # Check if it's a function call
        if self.current_token and self.current_token[0] == "LPAREN":
            # Move back to identifier to properly parse function call
            self.current_token_index -= 1
            self.current_token = self.tokens[self.current_token_index]

            # Parse function call and add to statements
            func_call = self.parse_function_call()
            self.statements.append({
                "type": "expression_statement",
                "expression": func_call
            })
            return

        # Otherwise, it's an assignment
        if not self.current_token or self.current_token[0] != "ASSIGN":
            self.statements.append({"type": "error", "value": f"Expected '=' after identifier {identifier}"})
            return

        self.advance_token()  # Move past ASSIGN

        # Parse the right-hand side of the assignment
        value = self.parse_expression()

        self.statements.append({
            "type": "assignment",
            "identifier": identifier,
            "value": value
        })

    def handle_numbers(self):
        """Convert Roman numerals to decimal"""
        if self.current_token[0] == "NUMBER":
            return RomanNumeralConverter.convert_roman_to_decimal(self.current_token[1])
        return 0

    def peek_next_token(self):
        """Look at the next token without advancing"""
        next_index = self.current_token_index + 1
        if next_index < len(self.tokens):
            return self.tokens[next_index]
        return None

    def datatype_mapping(self, datatype):
        """Map LISS datatypes to C datatypes"""
        mapping = {
            "INT": "int",
            "STRING": "char*",
            "CHAR": "char"
        }
        return mapping.get(datatype, "void")


    def conditions_mapping(self, condition):
        """Map LISS Conditions to C datatypes"""
        mapping = {
            "ISNOTEQUAL": "!=",
            "ISGREATER": "<",
            "ISLESS": ">"
        }
        return mapping.get(condition, "void")

    def generate_c_code(self):
        """Convert parsed statements to C code"""
        for statement in self.statements:
            if statement["type"] == "comment":
                self.c_code.append(f"// {statement['value']}")
            elif statement["type"] == "declaration":
                self.c_code.append(f"{statement['data_type']} {statement['identifier']};")
            elif statement["type"] == "declaration_assignment":
                value = self.format_expression(statement["value"])
                self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {value};")
            elif statement["type"] == "assignment":
                value = self.format_expression(statement["value"])
                self.c_code.append(f"{statement['identifier']} = {value};")
            elif statement["type"] == "expression_statement":
                expr = self.format_expression(statement["expression"])
                self.c_code.append(f"{expr};")
            elif statement["type"] == "print":
                value = self.format_expression(statement["expression"])
                format_specifier = self.get_format_specifier(statement["expression"]["type"])
                self.c_code.append(f"printf(\"{format_specifier}\", {value});")
            elif statement["type"] == "if_statement":
                self.generate_if_statement(statement)
            elif statement["type"] == "error":
                self.c_code.append(f"// ERROR: {statement['value']}")

    def generate_if_statement(self, if_statement):
        """Generate C code for if statements"""
        condition = self.format_condition(if_statement["condition"])

        # Add the if statement with condition
        self.c_code.append(f"if ({condition}) {{")

        # Add the body with indentation
        for stmt in if_statement["if_body"]:
            code = self.generate_single_statement(stmt)
            if code:
                self.c_code.append(f"    {code}")

        # Check for else body
        if if_statement["else_body"]:
            # Check if it's an else-if (first item is an if statement)
            if len(if_statement["else_body"]) == 1 and if_statement["else_body"][0].get("type") == "if_statement":
                # Generate else-if
                else_if = if_statement["else_body"][0]
                condition = self.format_condition(else_if["condition"])
                self.c_code.append(f"}} else if ({condition}) {{")

                # Add the else-if body with indentation
                for stmt in else_if["if_body"]:
                    code = self.generate_single_statement(stmt)
                    if code:
                        self.c_code.append(f"    {code}")

                # Handle nested else blocks recursively
                if else_if["else_body"]:
                    if len(else_if["else_body"]) == 1 and else_if["else_body"][0].get("type") == "if_statement":
                        # More else-if blocks to process
                        self.generate_if_statement(else_if)
                        # Skip closing brace as it will be added by the recursive call
                        return
                    else:
                        # Regular else block
                        self.c_code.append("} else {")
                        for stmt in else_if["else_body"]:
                            code = self.generate_single_statement(stmt)
                            if code:
                                self.c_code.append(f"    {code}")
            else:
                # Regular else block
                self.c_code.append("} else {")
                for stmt in if_statement["else_body"]:
                    code = self.generate_single_statement(stmt)
                    if code:
                        self.c_code.append(f"    {code}")

        # Close the if statement
        self.c_code.append("}")

    def generate_single_statement(self, statement):
        """Generate C code for a single statement"""
        if statement["type"] == "comment":
            return f"// {statement['value']}"
        elif statement["type"] == "declaration":
            return f"{statement['data_type']} {statement['identifier']};"
        elif statement["type"] == "declaration_assignment":
            value = self.format_expression(statement["value"])
            return f"{statement['data_type']} {statement['identifier']} = {value};"
        elif statement["type"] == "assignment":
            value = self.format_expression(statement["value"])
            return f"{statement['identifier']} = {value};"
        elif statement["type"] == "expression_statement":
            expr = self.format_expression(statement["expression"])
            return f"{expr};"
        elif statement["type"] == "print":
            value = self.format_expression(statement["expression"])
            format_specifier = self.get_format_specifier(statement["expression"]["type"])
            return f"printf(\"{format_specifier}\", {value});"
        elif statement["type"] == "error":
            return f"// ERROR: {statement['value']}"

        return ""

    def format_condition(self, condition):
        """Format a condition expression for C code generation"""
        if not condition:
            return ""

        if condition["type"] == "identifier" and "condition" in condition:
            # Handle comparison conditions
            left = condition["value"]
            operator = condition["condition"]
            right = condition.get("value2", "")
            return f"{left} {operator} {right}"
        else:
            # Handle other expressions
            return self.format_expression(condition)

    def format_expression(self, expr):
        """Format an expression for C code generation"""
        if not expr:
            return ""

        if expr["type"] == "number":
            return str(expr["value"])
        elif expr["type"] == "string":
            return f"\"{expr['value']}\""
        elif expr["type"] == "identifier":
            if "condition" in expr and "value2" in expr:
                # Handle condition expressions
                return f"{expr['value']} {expr['condition']} {expr['value2']}"
            return expr["value"]
        elif expr["type"] == "function_call":
            args = ", ".join([self.format_expression(arg) for arg in expr["arguments"]])
            return f"{expr['name']}({args})"
        return ""

    def get_format_specifier(self, expr_type):
        """Get printf format specifier based on expression type"""
        if expr_type == "number":
            return "%d"
        elif expr_type == "string":
            return "%s"
        elif expr_type == "char":
            return "%c"
        return "%s"