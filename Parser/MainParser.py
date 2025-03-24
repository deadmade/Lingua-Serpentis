from Parser.GenerateC import generate_c_code
from Parser.HelperFunctions import advance_token
from Parser.ParseDeclaration import parse_declaration
from Parser.ParseExpression import parse_function_call
from Parser.ParseLoops import parse_while_loop
from Parser.ParsePrint import parse_print
from Parser.ParseStatements import parse_normal_statement


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
            self.statements = parse_normal_statement(self, self.statements)
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
            statements.append({"type": "comment", "value": self.current_token[1]})
        elif token_type in ["INT", "STRING", "CHAR"]:
            statements.append(parse_declaration(self, statements))
        elif token_type == "PRINT":
            statements.append(parse_print(self, statements))
        elif token_type == "IF":
            statements.append(self.parse_if_statement(statements))
        elif token_type == "WHILE":
            statements.append(parse_while_loop(self, statements))
        elif token_type == "IDENTIFIER":
            function_call, new_statements = parse_function_call(self, statements)
            statements.append(function_call)
            if new_statements:
                statements.extend(new_statements)
        elif token_type == "NEWLINE" or token_type == "SEMICOLON":
            # Skip newlines and semicolons
            pass
        else:
            # Unknown token type
            statements.append({"type": "error", "value": f"Unknown token type: {token_type}"})

        return statements
