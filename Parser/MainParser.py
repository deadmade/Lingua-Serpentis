from Parser.GenerateC import generate_c_code
from Parser.HelperFunctions import advance_token, peek_next_token
from Parser.ParseDeclaration import parse_declaration, parse_assignment
from Parser.ParseExpression import parse_expression
from Parser.ParseExpression import parse_function_call
from Parser.ParsePrint import parse_print


class Parser:
    def __init__(self, tokens, is_test=False):
        self.tokens = tokens
        self.current_token_index = 0
        self.current_token = None
        self.c_code = []
        self.functions = {}
        self.statements = []
        self.is_test = is_test

        # Initialize with the first token if available
        if tokens:
            self.current_token = tokens[0]

    def parse(self):
        """Parse all tokens and return the generated C code"""
        while self.current_token_index < len(self.tokens):
            parsed = self.parse_statement(self.statements)
            if isinstance(parsed, list):
                self.statements.extend(parsed)
            elif parsed is not None:
                self.statements.append(parsed)

            advance_token(self)
        print(self.statements)

        # Convert statements to C code
        generate_c_code(self)
        return "\n".join(self.c_code)

    def parse_statement(self, statements):
        """Parse a single statement based on token type"""
        if not self.current_token:
            return

        token_type = self.current_token[0]

        if token_type == "COMMENT":
            return {"type": "comment", "value": self.current_token[1]}
        elif token_type == "FUNCTION":
            return self.parse_function_declaration(statements)
        elif token_type in ["INT", "STRING", "CHAR", "DOUBLE", "FLOAT"]:
            return parse_declaration(self, statements)
        elif token_type == "PRINT":
            return parse_print(self, statements)
        elif token_type == "IF":
            return self.parse_if_statement(statements)
        elif token_type == "WHILE":
            return self.parse_while_loop(statements)
        elif token_type == "FOR":
            return self.parse_for_loop(statements)
        elif token_type == "IDENTIFIER":
            if peek_next_token(self)[0] == "LPAREN":
                return parse_function_call(self, statements)
            elif peek_next_token(self)[0] == "ASSIGN":
                return parse_assignment(self, statements)
        elif token_type == "NEWLINE" or token_type == "SEMICOLON":
            # Skip newlines and semicolons
            pass
        else:
            # Unknown token type
            return {"type": "error", "value": f"Unknown token type: {token_type}"}

    def parse_loop_statement(self, statements):
        """Parse a single statement based on token type"""
        if not self.current_token:
            return

        token_type = self.current_token[0]

        if token_type == "BREAK":
            statements.append({"type": "break"})
        elif token_type == "CONTINUE":
            statements.append({"type": "continue"})
        else:
            # Delegate to parse_statement for other types
            statements.extend(self.parse_statement(statements))

        return statements

    def parse_function_statement(self, statements):
        """Parse a single statement based on token type"""
        if not self.current_token:
            return

        token_type = self.current_token[0]

        if token_type == "RETURN":
            advance_token(self)
            expr = parse_expression(self, statements)
            return [{"type": "return", "value": expr}]
        else:
            # Delegate to parse_statement for other types
            result = self.parse_statement(statements)
            return result if result else []

    def parse_while_loop(self, statements):
        """Parse a while loop with condition and body"""
        advance_token(self)  # Move past WHILE

        new_statements = []

        if not self.current_token or self.current_token[0] != "LPAREN":
            return {"type": "error", "value": "Expected '(' after while keyword"}

        advance_token(self)  # Move past LPAREN
        condition = parse_expression(self, new_statements)

        if not self.current_token or self.current_token[0] != "RPAREN":
            return {"type": "error", "value": "Expected ')' to close while condition"}

        advance_token(self)  # Move past RPAREN

        if not self.current_token or self.current_token[0] != "LBRACE":
            return {"type": "error", "value": "Expected '{' after while condition"}

        advance_token(self)  # Move past LBRACE
        body_statements = self.parse_block(statements, "loop")

        return {
            "type": "while_loop",
            "condition": condition,
            "body": body_statements
        }

    def parse_block(self, statements, block_type):
        """Parse a block of statements enclosed in braces"""
        block_statements = []

        # Parse statements until we find the closing brace
        while self.current_token and self.current_token[0] != "RBRACE":
            start_index = self.current_token_index
            current_statement = []
            if block_type == "function":
                new_statements = self.parse_function_statement(current_statement)
            elif block_type == "loop":
                new_statements = self.parse_loop_statement(current_statement)
            else:
                new_statements = self.parse_statement(current_statement)
            block_statements.extend([item for item in new_statements if item is not None])

            # If we didn't advance, manually advance to avoid infinite loop
            if self.current_token_index == start_index:
                advance_token(self)

        if not self.current_token or self.current_token[0] != "RBRACE":
            block_statements.append({"type": "error", "value": "Expected '}' to close block"})
            return block_statements

        advance_token(self)  # Move past RBRACE
        return block_statements

    def parse_if_statement(self, statements):
        """Parse an if statement with condition and body"""
        advance_token(self)  # Move past IF

        # Parse condition
        if not self.current_token or self.current_token[0] != "LPAREN":
            return {"type": "error", "value": "Expected '(' after if keyword"}

        advance_token(self)  # Move past LPAREN

        # Parse the condition expression
        condition = parse_expression(self, statements)

        if not self.current_token or self.current_token[0] != "RPAREN":
            return {"type": "error", "value": "Expected ')' to close if condition"}

        advance_token(self)  # Move past RPAREN

        # Parse body (statements between { and })
        if not self.current_token or self.current_token[0] != "LBRACE":
            return {"type": "error", "value": "Expected '{' after if condition"}

        advance_token(self)  # Move past LBRACE

        # Parse if body statements
        if_body = self.parse_block(statements, "if")

        # Check for else statement
        else_body = None
        if self.current_token and self.current_token[0] == "ELSE":
            advance_token(self)  # Move past ELSE

            # Check if it's an else if or a regular else
            if self.current_token and self.current_token[0] == "IF":
                # For else if, recursively parse another if statement
                else_body = [self.parse_if_statement(statements)]
            else:
                # For regular else, parse the block
                if not self.current_token or self.current_token[0] != "LBRACE":
                    return {"type": "error", "value": "Expected '{' after else keyword"}

                advance_token(self)  # Move past LBRACE
                else_body = self.parse_block(statements, "if")

        # Create and add the if statement to our statements list
        if_statement = {
            "type": "if_statement",
            "condition": condition,
            "if_body": if_body,
            "else_body": else_body
        }

        return if_statement

    def parse_for_loop(self, statements):
        """Parse a for loop with initialization, condition, increment, and body"""
        advance_token(self)  # Move past FOR

        if not self.current_token or self.current_token[0] != "LPAREN":
            return {"type": "error", "value": "Expected '(' after for keyword"}

        advance_token(self)  # Move past LPAREN

        # Parse initialization
        init = parse_declaration(self, statements)
        if not self.current_token or self.current_token[0] != "SEMICOLON":
            return {"type": "error", "value": "Expected ';' after for initialization"}

        advance_token(self)  # Move past SEMICOLON

        # Parse condition
        condition = parse_expression(self, statements)
        if not self.current_token or self.current_token[0] != "SEMICOLON":
            return {"type": "error", "value": "Expected ';' after for condition"}

        advance_token(self)  # Move past SEMICOLON

        # Parse increment
        increment = parse_assignment(self, statements)
        if not self.current_token or self.current_token[0] != "RPAREN":
            return {"type": "error", "value": "Expected ')' after for increment"}

        advance_token(self)  # Move past RPAREN

        if not self.current_token or self.current_token[0] != "LBRACE":
            return {"type": "error", "value": "Expected '{' after for condition"}

        advance_token(self)  # Move past LBRACE
        body_statements = self.parse_block(statements , "loop")

        for_loop = {
            "type": "for_loop",
            "init": init,
            "condition": condition,
            "increment": increment,
            "body": body_statements
        }

        return for_loop

    def parse_function_declaration(self, statements):
        """Parse a function declaration with return type, name, parameters, and body"""
        advance_token(self)  # Move past FUNCTION

        # Expect return type
        if self.current_token[0] not in ["INT", "STRING", "CHAR", "STRINGTYPE"]:
            return {"type": "error", "value": "Expected function name after return type"}
        return_type = self.current_token[1]
        advance_token(self)

        # Expect function name
        if self.current_token[0] != "IDENTIFIER":
            return {"type": "error", "value": "Expected function name after 'munus'"}
        func_name = self.current_token[1]
        advance_token(self)

        if self.current_token[0] != "LPAREN":
            return {"type": "error", "value": "Expected '(' after function name"}
        advance_token(self)

        # Parse parameters
        params = []
        while self.current_token and self.current_token[0] != "RPAREN":
            # Check for valid parameter type tokens
            if self.current_token[0] not in ["INT", "STRING", "CHAR"]:
                return {"type": "error", "value": f"Invalid parameter type: {self.current_token[1]}"}
            param_type = self.current_token[1]
            advance_token(self)

            if self.current_token[0] != "IDENTIFIER":
                return {"type": "error", "value": f"Expected parameter name after type '{param_type}'"}
            param_name = self.current_token[1]
            params.append((param_type, param_name))  # <-- HIER ist der wichtige Fix
            advance_token(self)

            if self.current_token and self.current_token[0] == "COMMA":
                advance_token(self)  # Skip comma

        if self.current_token[0] != "RPAREN":
            return {"type": "error", "value": "Expected ')' after parameter list"}
        advance_token(self)

        if self.current_token[0] != "LBRACE":
            return {"type": "error", "value": "Expected '{' to start function body"}
        advance_token(self)

        body_statements = self.parse_block(statements, "function")

        func_data = {
            "type": "function_declaration",
            "name": func_name,
            "params": params,
            "body": body_statements,
            "return_type": return_type
        }

        self.functions[func_name] = func_data
        return func_data

