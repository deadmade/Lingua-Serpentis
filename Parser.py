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
            self.statements = self.parse_statement(self.statements)
            self.advance_token()

        print(self.statements)

        # Convert statements to C code
        self.generate_c_code()
        return "\n".join(self.c_code)

    def advance_token(self):
        """Move to the next token"""
        self.current_token_index += 1
        if self.current_token_index < len(self.tokens):
            self.current_token = self.tokens[self.current_token_index]
        else:
            self.current_token = None

    def parse_statement(self, statements):
        """Parse a single statement based on token type"""
        if not self.current_token:
            return

        token_type = self.current_token[0]

        if token_type == "COMMENT":
            statements.append({"type": "comment", "value": self.current_token[1]})
        elif token_type in ["INT", "STRING", "CHAR"]:
            statements.append(self.parse_declaration(statements))
        elif token_type == "PRINT":
            statements.append(self.parse_print(statements))
        elif token_type == "IF":
            statements.append(self.parse_if_statement(statements))
        elif token_type == "WHILE":
           statements.append(self.parse_while_loop(statements))
        elif token_type == "BREAK":
            statements.append({"type": "break"})
        elif token_type == "CONTINUE":
            statements.append({"type": "continue"})
        elif token_type == "IDENTIFIER":
            statements.append(self.parse_assignment_or_function_call(statements))
        elif token_type == "NEWLINE" or token_type == "SEMICOLON":
            # Skip newlines and semicolons
            pass
        else:
            # Unknown token type
            statements.append({"type": "error", "value": f"Unknown token type: {token_type}"})

        return statements

    def parse_declaration(self, statements):
        """Parse variable declaration and assignment"""
        data_type = self.datatype_mapping(self.current_token[0])
        self.advance_token()

        if not self.current_token or self.current_token[0] != "IDENTIFIER":
            statements.append({"type": "error", "value": "Expected identifier after type declaration"})
            return

        identifier = self.current_token[1]
        self.advance_token()

        # Simple declaration without assignment
        if not self.current_token or self.current_token[0] in ["NEWLINE", "SEMICOLON"]:
            statements.append({
                "type": "declaration",
                "data_type": data_type,
                "identifier": identifier
            })
            return

        # Declaration with assignment
        if self.current_token[0] == "ASSIGN":
            self.advance_token()
            value, type_operation, statements_added = self.parse_expression(statements)

            if statements_added is not None:
                statements.append(statements_added)

            statements.append({
                "type": type_operation,
                "data_type": data_type,
                "identifier": identifier,
                "value": value
            })
        return statements

    def parse_expression(self, statements):
        """Parse an expression (number, identifier, function call)"""
        if not self.current_token:
            return None, None, None

        token_type = self.current_token[0]
        next_token = self.peek_next_token()

        # Handle number literal
        if token_type == "NUMBER":
            value = self.handle_numbers()
            self.advance_token()

            # Check if this is part of an operation
            if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV"]:
                condition = self.conditions_mapping(self.current_token[0])
                self.advance_token()

                # Parse the right side of the operation
                if self.current_token and self.current_token[0] == "NUMBER":
                    value2 = self.handle_numbers()
                    self.advance_token()
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
            self.advance_token()
            return {"type": "string", "value": value}, "declaration_assignment", None

        # Handle identifiers
        elif token_type == "IDENTIFIER":
            identifier = self.current_token[1]
            self.advance_token()

            # Check if this is a function call
            if self.current_token and self.current_token[0] == "LPAREN":
                self.current_token_index -= 1  # Go back to parse the function call properly
                self.current_token = self.tokens[self.current_token_index]
                function_call, statements_added = self.parse_function_call(statements)
                return function_call, "declaration_function_call" , statements_added

            # Check if this is part of an operation
            if self.current_token and self.current_token[0] in ["PLUS", "MINUS", "MULT", "DIV", "ISNOTEQUAL",
                                                                "ISGREATER",
                                                                "ISLESS"]:
                condition = self.conditions_mapping(self.current_token[0])
                self.advance_token()

                # Parse the right side of the operation
                if self.current_token and self.current_token[0] == "NUMBER":
                    value2 = self.handle_numbers()
                    self.advance_token()
                elif self.current_token and self.current_token[0] == "IDENTIFIER":
                    value2 = self.current_token[1]
                    self.advance_token()
                else:
                    value2 = None

                return {"type": "operation", "value": identifier, "condition": condition,
                        "value2": value2}, "declaration_operation", None

            return {"type": "identifier", "value": identifier}, "declaration_assignment", None

        return None, None, None


    def parse_function_call(self, statements):
        """Parse a function call with arguments"""
        function_name = self.current_token[1]
        self.advance_token()  # Move to LPAREN

        if not self.current_token or self.current_token[0] != "LPAREN":
            statements.append({"type": "error", "value": f"Expected '(' after function name {function_name}"})
            return None, statements

        self.advance_token()  # Move past LPAREN

        arguments = []
        # Parse arguments until we find the closing parenthesis
        while self.current_token and self.current_token[0] != "RPAREN":
            arg, _, statements = self.parse_expression(statements)
            if arg:
                arguments.append(arg)

            # Skip commas between arguments
            if self.current_token and self.current_token[0] == "COMMA":
                self.advance_token()

        if not self.current_token or self.current_token[0] != "RPAREN":
            statements.append(
            {"type": "error", "value": f"Expected ')' to close function call to {function_name}"})
            return None, statements

        self.advance_token()  # Move past RPAREN

        return {
        "type": "function_call",
        "name": function_name,
        "arguments": arguments
        }, statements


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


    def parse_while_loop(self, statements):
        """Parse a while loop with condition and body"""
        self.advance_token()  # Move past WHILE

        if not self.current_token or self.current_token[0] != "LPAREN":
            statements.append({"type": "error", "value": "Expected '(' after while keyword"})
            return statements

        self.advance_token()  # Move past LPAREN
        condition = self.parse_expression()

        if not self.current_token or self.current_token[0] != "RPAREN":
            statements.append({"type": "error", "value": "Expected ')' to close while condition"})
            return statements

        self.advance_token()  # Move past RPAREN

        if not self.current_token or self.current_token[0] != "LBRACE":
            statements.append({"type": "error", "value": "Expected '{' after while condition"})
            return statements

        self.advance_token()  # Move past LBRACE
        body_statements = self.parse_block()

        statements.append({
            "type": "while_loop",
            "condition": condition,
            "body": body_statements
        })

        return statements


    def parse_block(self, statements):
        """Parse a block of statements enclosed in braces"""
        block_statements = []

        # Parse statements until we find the closing brace
        while self.current_token and self.current_token[0] != "RBRACE":
            start_index = self.current_token_index
            current_statement = self.statements
            statements = self.parse_statement(statements)

            # If we didn't advance, manually advance to avoid infinite loop
            if self.current_token_index == start_index:
                self.advance_token()

            # Get the last parsed statement if available
            if statements and self.statements != current_statement:
                block_statements.append(self.statements.pop())

        if not self.current_token or self.current_token[0] != "RBRACE":
            self.statements.append({"type": "error", "value": "Expected '}' to close block"})
            return block_statements

        self.advance_token()  # Move past RBRACE
        return block_statements


    def parse_assignment_or_function_call(self, statements):
        """Parse either an assignment or a function call"""
        identifier = self.current_token[1]
        self.advance_token()

        # Check if it's a function call
        if self.current_token and self.current_token[0] == "LPAREN":
            # Move back to identifier to properly parse function call
            self.current_token_index -= 1
            self.current_token = self.tokens[self.current_token_index]

            # Parse function call and add to statements
            func_call = self.parse_function_call(statements)
            statements.append({
                "type": "expression_statement",
                "expression": func_call
            })
            return statements

        # Otherwise, it's an assignment
        if not self.current_token or self.current_token[0] != "ASSIGN":
            statements.append({"type": "error", "value": f"Expected '=' after identifier {identifier}"})
            return statements

        self.advance_token()  # Move past ASSIGN

        # Parse the right-hand side of the assignment
        value = self.parse_expression(statements)

        statements.append({
            "type": "assignment",
            "identifier": identifier,
            "value": value
        })

        return statements


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
            "ISGREATER": ">",
            "ISLESS": "<",
            "PLUS": "+",
            "MINUS": "-",
            "MULT": "*",
            "DIV": "/",
            "minor_est": "<",
            "maior_est": ">",
            "par_est": "=="
        }
        return mapping.get(condition, "void")


    def generate_c_code(self):
        """Convert parsed statements to C code"""
        #self.c_code.append("#include <stdio.h>")
        #self.c_code.append("int main() {")
        for statement in self.statements:
            if statement["type"] == "comment":
                self.c_code.append(f"// {statement['value']}")
            elif statement["type"] == "declaration":
                self.c_code.append(f"{statement['data_type']} {statement['identifier']};")
            elif statement["type"] == "declaration_function_call":
                expr = self.format_expression(statement)
                self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {expr};")
            elif statement["type"] == "declaration_assignment":
                value = self.format_expression(statement["value"])
                self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {value};")
            elif statement["type"] == "declaration_operation":
                value = self.format_expression(statement["value"])
                self.c_code.append(f"{statement['data_type']} {statement['identifier']} = {value};")
            elif statement["type"] == "assignment":
                value = self.format_expression(statement["value"])
                self.c_code.append(f"{statement['identifier']} = {value};")
            elif statement["type"] == "expression_statement":
                expression = statement["expression"][0]
                if expression["type"] == "function_call":
                    expr = self.format_expression(expression)
                    self.c_code.append(f"{expr};")
                else:
                    expr = self.format_expression(statement["expression"])
                    self.c_code.append(f"{expr};")
            elif statement["type"] == "print":
                value = self.format_expression(statement["expression"])
                format_specifier = self.get_format_specifier(statement["expression"]["type"])
                self.c_code.append(f"printf(\"{format_specifier}\", {value});")
            elif statement["type"] == "if_statement":
                self.generate_if_statement(statement)
            elif statement["type"] == "while_loop":
                self.generate_while_loop(statement)
            elif statement["type"] == "break":
                self.c_code.append("break;")
            elif statement["type"] == "continue":
                self.c_code.append("continue;")
            elif statement["type"] == "error":
                print(f"// ERROR: {statement['value']}")
        #self.c_code.append("    return 0;")
        #self.c_code.append("}")


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


    def generate_while_loop(self, while_statement):
        """Generate C code for while loops"""
        condition, _ = while_statement["condition"]  # Unpack the tuple
        formatted_condition = self.format_condition(condition)
        self.c_code.append(f"while ({formatted_condition}) {{")
        for stmt in while_statement["body"]:
            code = self.generate_single_statement(stmt)
            if code:
                self.c_code.append(f"    {code}")
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
        elif statement["type"] == "break":
             return "break;"
        elif statement["type"] == "continue":
             return "continue;"
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
      elif expr["type"] == "operation":
          value = expr["value"]
          value2 = expr["value2"]
          condition = expr["condition"]
          return f"{value} {condition} {value2}"
      elif expr["type"] == "string":
          return f"\"{expr['value']}\""
      elif expr["type"] == "identifier":
          if "condition" in expr and "value2" in expr:
              # Handle condition expressions
              return f"{expr['value']} {expr['condition']} {expr['value2']}"
          return expr["value"]
      elif expr["type"] == "declaration_function_call":
        value = expr["value"]
        args = [self.format_expression(arg) for arg in value["arguments"]]
        return f"{value['name']}({', '.join(args)})"
      elif expr["type"] == "function_call":
          args = [self.format_expression(arg) for arg in expr["arguments"]]
          return f"{expr['name']}({', '.join(args)})"





    def get_format_specifier(self, expr_type):
        """Get printf format specifier based on expression type"""
        if expr_type == "number":
            return "%d"
        elif expr_type == "string":
            return "%s"
        elif expr_type == "char":
            return "%c"
        return "%s"
