class Parser:
    def __init__(self, tokens):
        self.tokens = tokens
        self.current = 0
        self.python_code = ""

    # Top level parsing method
    def parse(self):
        """Parse the program and return Python code"""
        while self.current < len(self.tokens):
            self.python_code += self.statement()
        return self.python_code

    # Statement parsing methods
    def statement(self):
        """Parse a statement"""
        # This will dispatch to specific statement types
        if self.match("IF"):
            return self.if_statement()
        elif self.match("WHILE"):
            return self.while_statement()
        elif self.match("FOR"):
            return self.for_statement()
        elif self.match("PRINT"):
            return self.print_statement()
        else:
            return self.expression_statement()

    def if_statement(self):
        """Parse an if statement"""
        # TODO: Implement for Latin syntax
        condition = self.expression()
        then_branch = self.statement()

        result = f"if {condition}:\n{then_branch}"

        if self.match("ELSE"):
            else_branch = self.statement()
            result += f"else:\n{else_branch}"

        return result

    def while_statement(self):
        """Parse a while loop"""
        condition = self.expression()
        body = self.statement()

        return f"while {condition}:\n{body}"

    def for_statement(self):
        """Parse a for loop"""
        # Assuming a for loop similar to Python's for i in range() structure
        self.match("LPAREN")
        var = self.match("IDENTIFIER")[1]
        self.match("IN")
        iterable = self.expression()
        self.match("RPAREN")
        body = self.statement()

        return f"for {var} in {iterable}:\n{body}"

    def print_statement(self):
        """Parse a print statement"""
        self.match("LPAREN")
        value = self.expression()
        self.match("RPAREN")

        return f"print({value})\n"

    def expression_statement(self):
        """Parse an expression statement"""
        expr = self.expression()
        return f"{expr}\n"

    # Expression parsing methods - follows precedence rules
    def expression(self):
        """Parse an expression"""
        return self.assignment()

    def assignment(self):
        """Parse an assignment expression"""
        expr = self.logical_or()

        if self.match("ASSIGN"):
            value = self.assignment()
            if isinstance(expr, str):  # Should be a variable name
                return f"{expr} = {value}"
            else:
                raise Exception("Invalid assignment target")

        return expr

    def logical_or(self):
        """Parse a logical OR expression"""
        expr = self.logical_and()

        while self.match("OR"):
            right = self.logical_and()
            expr = f"{expr} or {right}"

        return expr

    def logical_and(self):
        """Parse a logical AND expression"""
        expr = self.equality()

        while self.match("AND"):
            right = self.equality()
            expr = f"{expr} and {right}"

        return expr

    def equality(self):
        """Parse an equality expression"""
        expr = self.comparison()

        while self.match("EQUAL", "NOT_EQUAL"):
            operator = "==" if self.tokens[self.current - 1][0] == "EQUAL" else "!="
            right = self.comparison()
            expr = f"{expr} {operator} {right}"

        return expr

    def comparison(self):
        """Parse a comparison expression"""
        expr = self.term()

        while self.match("GREATER", "GREATER_EQUAL", "LESS", "LESS_EQUAL"):
            operator = self.tokens[self.current - 1][1]
            right = self.term()
            expr = f"{expr} {operator} {right}"

        return expr

    def term(self):
        """Parse a term expression"""
        expr = self.factor()

        while self.match("PLUS", "MINUS"):
            operator = self.tokens[self.current - 1][1]
            right = self.factor()
            expr = f"{expr} {operator} {right}"

        return expr

    def factor(self):
        """Parse a factor expression"""
        expr = self.primary()

        while self.match("MULT", "DIV", "MOD"):
            operator = self.tokens[self.current - 1][1]
            if operator == "%":
                operator = "%"  # MOD becomes % in Python
            right = self.primary()
            expr = f"{expr} {operator} {right}"

        return expr

    def primary(self):
        """Parse a primary expression"""
        if self.match("FALSE"):
            return "False"
        if self.match("TRUE"):
            return "True"
        if self.match("NULL"):
            return "None"

        if self.match("NUMBER"):
            return self.tokens[self.current - 1][1]

        if self.match("STRING"):
            return self.tokens[self.current - 1][1]

        if self.match("IDENTIFIER"):
            return self.tokens[self.current - 1][1]

        if self.match("LPAREN"):
            expr = self.expression()
            self.match("RPAREN")
            return f"({expr})"

        raise Exception(f"Unexpected token: {self.peek()}")

    # Helper methods for token management
    def peek(self):
        """Look at the current token without consuming it"""
        if self.current >= len(self.tokens):
            return (None, None)
        return self.tokens[self.current]

    def advance(self):
        """Consume the current token and return it"""
        token = self.peek()
        self.current += 1
        return token

    def match(self, *token_types):
        """Check if current token matches any of the given types"""
        current_token = self.peek()
        if current_token[0] in token_types:
            return self.advance()
        return None