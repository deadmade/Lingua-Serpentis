from Parser.GenerateC import generate_c_code
from Parser.HelperFunctions import advance_token
from Parser.ParseDeclaration import parse_declaration
from Parser.ParseExpression import parse_expression
from Parser.ParseExpression import parse_function_call
from Parser.ParsePrint import parse_print


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
            self.statements.extend(self.parse_statement(self.statements))
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
        new_statements = []

        if token_type == "COMMENT":
            new_statements.append({"type": "comment", "value": self.current_token[1]})
        elif token_type in ["INT", "STRING", "CHAR"]:
            new_statements.append(parse_declaration(self, statements))
        elif token_type == "PRINT":
            new_statements.append(parse_print(self, statements))
        elif token_type == "IF":
            new_statements.append(self.parse_if_statement(statements))
        elif token_type == "WHILE":
            new_statements.append(self.parse_while_loop(statements))
        elif token_type == "IDENTIFIER":
            function_call, new_statements = parse_function_call(self, statements)
            new_statements.append(function_call)
            if new_statements:
                new_statements.extend(new_statements)
        elif token_type == "NEWLINE" or token_type == "SEMICOLON":
            # Skip newlines and semicolons
            pass
        else:
            # Unknown token type
            new_statements.append({"type": "error", "value": f"Unknown token type: {token_type}"})

        return new_statements

    def parse_while_loop(self, statements):
        """Parse a while loop with condition and body"""
        advance_token(self)  # Move past WHILE

        new_statements = []

        if not self.current_token or self.current_token[0] != "LPAREN":
            new_statements.append({"type": "error", "value": "Expected '(' after while keyword"})
            return new_statements

        advance_token(self)  # Move past LPAREN
        condition = parse_expression(self, new_statements)
        advance_token(self)

        if not self.current_token or self.current_token[0] != "RPAREN":
            new_statements.append({"type": "error", "value": "Expected ')' to close while condition"})
            return new_statements

        advance_token(self)  # Move past RPAREN

        if not self.current_token or self.current_token[0] != "LBRACE":
            new_statements.append({"type": "error", "value": "Expected '{' after while condition"})
            return new_statements

        advance_token(self)  # Move past LBRACE
        body_statements = self.parse_block(statements)

        statements.append({
            "type": "while_loop",
            "condition": condition,
            "body": body_statements
        })

        return statements

    # TODO: Das hier ist noch nicht fertig
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

    def parse_block(self, statements):
        """Parse a block of statements enclosed in braces"""
        block_statements = []

        # Parse statements until we find the closing brace
        while self.current_token and self.current_token[0] != "RBRACE":
            start_index = self.current_token_index
            current_statement = []
            new_statements = self.parse_loop_statement(current_statement)
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
        if_body = self.parse_block(statements)

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
                else_body = self.parse_block(statements)

        # Create and add the if statement to our statements list
        if_statement = {
            "type": "if_statement",
            "condition": condition,
            "if_body": if_body,
            "else_body": else_body
        }

        return if_statement
